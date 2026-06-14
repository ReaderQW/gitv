```
giter-app/
├── Cargo.toml           # tauri + rfd + serde
├── build.rs             # tauri_build::build()
├── tauri.conf.json      # Tauri 配置 (窗口大小、全局 API)
├── icons/icon.ico       # 应用图标 (Python 生成的 32x32)
├── src/
│   ├── main.rs          # 入口: → giter_app_lib::run()
│   ├── lib.rs           # Tauri builder + 8 个命令
│   └── state/
│       ├── mod.rs
│       ├── app_settings.rs   # 配置持久化 (JSON)
│       ├── selected_repos.rs # 仓库路径集合
│       └── scan_result.rs    # 扫描状态机
└── frontend/
    ├── index.html   # 布局: 工具栏 + 侧栏仓库列表 + 主内容区
    ├── style.css    # 现代深色工具栏 + 卡片统计
    └── app.js       # Tauri IPC 调用 + DOM 渲染
```
