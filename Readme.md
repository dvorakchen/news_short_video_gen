# News Short Video Gen

Generate news short video of PengPai News

![screenshot]('./screenshot.png')

## Env

Please create a `.env` file

```sh
OPENAI_KEY=<DeepSeek API Key>
OPENAI_BASE_URL=<DeepSeek API URL> https://api.deepseek.com
TTS_URL=<Ali TTS Server URL> https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation
ALI_DASHSCOPE_API_KEY=Ali API Key
```

## Tool chains

You need:

- Rust 1.88
- Bun
- Tauri
- FFmpeg

## Dev

```sh
cd ndclient

bun i
bun tauri dev
```

## Build

```sh
cd ndclient

bun i
bun tauri build
```