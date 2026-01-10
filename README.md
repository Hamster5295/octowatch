# Octowatch

Octowatch 是基于 [Kovi](https://kovi.thricecola.com/) 框架的 Github 仓库动态检查与总结插件，可以定期检查 Github 仓库动态，对进度进行总结，以及在项目停滞的时候进行激励(?)

## 安装

1. 根据[教程](https://kovi.thricecola.com/start/fast.html)创建一个 Kovi 工程
2. 在项目根目录运行
```bash
cargo add kovi-plugin-octowatch
```

3. 在 `build_bot!` 宏中传入插件
```rust
let bot = build_bot!(kovi-plugin-octowatch /* 和其他你正在使用的插件，用 , 分割 */ );
```

## 配置

octowatch 可以通过 `toml` 文件进行配置。如果配置文件不存在，则使用默认配置。  

```toml
# 可以访问到仓库的 Token
github_token = "github_pat_xxxxxx"

# 使用的大语言模型 API
[llm]
url = "https://someapi/v1/chat/completions"
key = "Some-Api-Key"
model = "Some/AwesomeModel"

# 总结与鼓励的 Prompt
prompt_summary = "以下给出了一个 github 仓库在一段时间内的提交记录，请以活泼可爱的语言在40字以内给出一个总结，不要使用任何 Markdown 格式。"
prompt_criticize = "在过去一段时间里，一个 github 仓库没有任何提交记录。请你用活泼可爱的语言激励这个项目的开发者，鼓励他们积极提交代码。你一定不能使用Markdown格式。"

# 需要检查的仓库们
[[repos]]
groups = [12345678]         # 检查内容将发送到这些群聊
time = "0 0 23"             # Crons 字符串，指定检查仓库的时间与间隔，本示例为每天 23 点检查一次
interval = 24               # 只总结距离当前时间 X 小时的提交记录，本示例为检查 24 小时以内的提交
owner = "XXX"               # 仓库所有者
repo = "YYY"                # 仓库名称


```  

配置文件应放置于编译后与可执行文件同级的 `data/kovi-plugin-octowatch/config.toml` 中。