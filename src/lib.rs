mod config;

use std::{collections::HashMap, sync::Arc};

use kovi::{
    PluginBuilder as plugin, RuntimeBot, Segment, bot::runtimebot::kovi_api::SetAccessControlList, chrono::{Duration, Utc}, event::id::ID, log::{info, warn}, serde_json::json,
};
use kovi_onebot::*;
use octocrab::Octocrab;
use openai::chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole};

use crate::config::RepoConfig;

const PLUGIN_NAME: &str = "kovi-plugin-octowatch";

#[kovi::plugin]
async fn main() {
    let bot = plugin::get_runtime_bot();
    let config = config::init(bot.get_data_path()).await.unwrap();

    bot.set_plugin_access_control(PLUGIN_NAME, true).unwrap();
    bot.set_plugin_access_control_list(
        PLUGIN_NAME,
        true,
        SetAccessControlList::Adds(
            config
                .repos
                .iter()
                .cloned()
                .flat_map(|r| r.groups)
                .map(|i| ID::new(i))
                .collect(),
        ),
    )
    .unwrap();

    let mut gh = octocrab::OctocrabBuilder::new();
    if let Some(token) = &config.github_token {
        gh = gh.user_access_token(token.clone());
    }
    let gh = Arc::new(gh.build().unwrap());

    for repo in &config.repos {
        plugin::cron(&format!("{}/{} * * ?", repo.time, repo.interval), {
            let bot = bot.clone();
            let gh = gh.clone();
            move || handle_repo_check(repo, bot.clone(), gh.clone())
        })
        .unwrap();
    }

    info!("[octowatch] Ready to watch some github repos!")
}

struct Contribution {
    author: String,
    commits: Vec<String>,
}

async fn handle_repo_check(repo: &RepoConfig, bot: Arc<RuntimeBot>, gh: Arc<Octocrab>) {
    let conf = config::CONFIG.get().unwrap();

    let now: kovi::chrono::DateTime<Utc> = Utc::now();
    let commits = gh
        .repos(&repo.owner, &repo.repo)
        .list_commits()
        .since(now - Duration::hours(repo.interval.into()))
        .send()
        .await;

    if let Err(e) = commits {
        warn!("[octowatch] Failed to fetch commits: {e}");
        return;
    }

    let commits = commits.unwrap();
    let cnt = commits.items.len();

    info!(
        "[octowatch] Retrived {} commit(s) from {}/{}",
        cnt, repo.owner, repo.repo
    );

    let mut conts: HashMap<String, Contribution> = HashMap::new();
    for commit in commits {
        let author = match commit.commit.author {
            Some(c) => c,
            None => {
                info!(
                    "[octowatch] Commit {} has no author, skipped",
                    &commit.sha[0..6]
                );
                continue;
            }
        };

        let email = author.email;
        if email.is_none() {
            continue;
        }
        let email = email.unwrap();

        if !conts.contains_key(&email) {
            conts.insert(
                email.clone(),
                Contribution {
                    author: author.name,
                    commits: vec![],
                },
            );
        }

        let cont = conts.get_mut(&email).unwrap();
        let msg = commit.commit.message.trim().to_string();

        let msg = if let Some(idx) = msg.find('\n') {
            let is_merge = msg.starts_with("Merge");

            if is_merge {
                format!("[Merge] {}", msg[idx + 1..].trim())
            } else {
                msg[..idx].trim().to_string()
            }
        } else {
            commit.commit.message
        };
        cont.commits.push(msg.trim().to_string());
    }

    info!(
        "[octowatch] {} user has contributed, gathered.",
        conts.len()
    );

    let mut prompts = vec![];

    if !conts.is_empty() {
        prompts.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: Some(conf.llm.prompt_summary.clone()),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
        });
        prompts.extend(
            conts
                .values()
                .flat_map(|e| &e.commits)
                .map(|e| ChatCompletionMessage {
                    role: ChatCompletionMessageRole::User,
                    content: Some(e.clone()),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                }),
        );
    } else {
        prompts.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: Some(conf.llm.prompt_criticize.clone()),
            name: None,
            function_call: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    let cmpl = ChatCompletion::builder(&conf.llm.model, prompts)
        .credentials(conf.llm.cred.clone())
        .create()
        .await;

    if let Err(e) = cmpl {
        warn!("[octowatch] Failed to create LLM completion: {e}");
        return;
    }

    let cmpl = cmpl.unwrap().choices[0].message.content.clone();

    if cmpl.is_none() {
        warn!("[octowatch] No content returned from LLM");
        return;
    }

    let cmpl = cmpl.unwrap();
    let cmpl = if cmpl.contains("</think>") {
        cmpl.split("</think>").nth(1).unwrap().to_string()
    } else {
        cmpl
    };

    let mut txts: Vec<String> = vec![
        format!("仓库 {}/{}", repo.owner, repo.repo),
        format!(
            "在过去的 {} 小时里共接收到 {} 次 commit\n",
            repo.interval, cnt
        ),
        cmpl.trim().to_string(),
    ];

    if !conts.is_empty() {
        txts.push("".into());
        txts.push("各成员贡献情况:\n".into());
    }

    let mut msgs: Vec<Segment> = vec![Segment::new(
        "text",
        json!(
            {
                "text": txts.join("\n")
            }
        ),
    )];

    for (usr, cont) in conts {
        let head = if !usr.ends_with("qq.com") {
            let u = cont.author;
            Segment::new(
                "text",
                json!({
                    "text":u
                }),
            )
        } else {
            let qq = usr.split('@').next().unwrap();

            match qq.parse::<u32>().ok() {
                Some(qq) => {
                    info!("[octowatch] Extracted QQ: {}", qq);

                    Segment::new(
                        "at",
                        json!({
                           "qq":qq
                        }),
                    )
                }
                None => {
                    let u = cont.author;
                    Segment::new(
                        "text",
                        json!({
                            "text":u
                        }),
                    )
                }
            }
        };

        msgs.push(head);

        let cmts = cont
            .commits
            .iter()
            .map(|msg| format!("- {msg}"))
            .collect::<Vec<String>>()
            .join("\n");

        msgs.push(Segment::new(
            "text",
            json!({
                "text":format!("\n{}\n\n", cmts)
            }),
        ));
    }

    for g in &repo.groups {
        bot.send_group_msg(g.to_owned(), msgs.clone());
    }
}
