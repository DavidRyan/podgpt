# PodGPT

A Discord bot powered by GPT that brings AI conversations, image generation, and more to your server — built in Rust for speed and reliability.

## Features

**Chat with GPT** — Start conversations with `/ask` or continue them with `/reply`. PodGPT maintains per-user conversation history so context flows naturally across messages.

**Vision** — Attach images directly to your messages or let PodGPT automatically pick up recently posted images in the channel. GPT sees what you see.

**Image Generation** — Generate images on the fly with `/image` and a text prompt. Results are posted directly as Discord attachments.

**Roast Mode** — Use `/roast @someone` and let GPT cook. It reads the room (channel context) before delivering the goods.

**Tool Use** — GPT can autonomously call tools mid-conversation:
- **URL Reader** — Drop a link and GPT will fetch and read the page content, with special handling for Twitter/X posts via the fxtwitter API.
- **Channel Summary** — GPT can read recent channel history to answer questions about what's been discussed.

**Conversation Management** — `/history` to review your conversation, `/conversations` to see a summary, `/clear` to start fresh.

## Commands

| Command | Description |
|---------|-------------|
| `/ask <prompt>` | Start a new conversation with GPT |
| `/reply <prompt>` | Continue your existing conversation |
| `/image <prompt>` | Generate an image from a text description |
| `/roast @user` | Roast a server member |
| `/history [count]` | View recent conversation messages |
| `/conversations` | See a summary of your active conversation |
| `/clear` | Clear your conversation history |

## Setup

### Prerequisites

- [Rust](https://rustup.rs/) (2024 edition)
- A [Discord bot token](https://discord.com/developers/applications)
- An [OpenAI API key](https://platform.openai.com/api-keys)

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DISCORD_TOKEN` | Yes | — | Discord bot token |
| `OPENAI_API_KEY` | Yes | — | OpenAI API key |
| `OPENAI_MODEL` | No | `gpt-4o` | Model to use for chat |
| `MAX_TOKENS` | No | `2048` | Max tokens per response |
| `SYSTEM_PROMPT` | No | *(built-in)* | Custom system prompt |
| `TAVILY_API_KEY` | No | — | Tavily API key for web search |

### Run

```bash
export DISCORD_TOKEN="your-token"
export OPENAI_API_KEY="your-key"

cargo run
```

## Architecture

```
src/
├── main.rs                  # Entry point
├── bot.rs                   # Discord framework setup & command registry
├── config.rs                # Environment-based configuration
├── error.rs                 # Unified error handling
├── utils.rs                 # Message splitting & utilities
├── commands/
│   ├── chat.rs              # /ask, /reply — chat with vision support
│   ├── image.rs             # /image — image generation
│   ├── manage.rs            # /clear, /history, /conversations
│   └── roast.rs             # /roast — targeted roasts
└── services/
    ├── chat.rs              # ChatService — conversation orchestration & tool dispatch
    ├── conversation.rs      # Conversation & message data structures
    ├── image.rs             # ImageService — OpenAI image generation
    ├── tools.rs             # Tool trait — extensible tool system for GPT
    ├── url_reader.rs        # URL fetching, HTML stripping, Twitter/X support
    └── channel_summary.rs   # Discord channel history reading
```

Built with [poise](https://github.com/serenity-rs/poise) + [serenity](https://github.com/serenity-rs/serenity) for Discord, and [async-openai](https://github.com/64bit/async-openai) for the OpenAI API.

## Extending

Adding a new tool is straightforward — implement the `Tool` trait in `src/services/tools.rs`:

```rust
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "What this tool does" }
    fn parameters(&self) -> serde_json::Value { /* JSON Schema */ }
    fn execute(&self, arguments: &str) -> Pin<Box<dyn Future<Output = String> + Send + '_>> {
        // Your logic here
    }
}
```

Then register it in `bot.rs`:

```rust
chat.register_tool(MyTool::new());
```

GPT will automatically decide when to use it based on the description and conversation context.

## License

MIT
