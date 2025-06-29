# DizzySync - Dizzylab自动同步器

一个用Rust编写的Dizzylab音乐自动同步工具，可以自动下载你购买的所有专辑。

## 特性

- 🎵 自动同步所有已购买的专辑
- 📁 支持多种文件组织方式（铺平/子文件夹）
- 🎛️ 支持多种音质格式（128kbps/320kbps MP3/FLAC/特典）
- ⚡ 单线程下载，避免给服务器造成压力
- 🔧 基于TOML的简单配置
- 🖥️ 命令行界面（未来将支持GUI）

## 安装

### 从源码编译

1. 确保安装了Rust (1.70+)
2. 克隆仓库并编译：

```bash
git clone https://github.com/Ovler-Young/dizzysync.git
cd dizzysync
cargo build --release
```

编译完成后，可执行文件位于 `target/release/dizzysync`

## 使用方法

### 1. 创建配置文件

```bash
./dizzysync --init
```

这会创建一个默认的 `config.toml` 配置文件。

### 2. 配置Cookie

编辑 `config.toml` 文件，设置你的Dizzylab cookie：

1. 在浏览器中登录 Dizzylab
2. 打开开发者工具 (F12)
3. 在网络选项卡中找到任意请求
4. 复制Cookie头的值
5. 将cookie值粘贴到配置文件中

Cookie格式示例：
```
sessionid=your_session_id_here; csrftoken=your_csrf_token_here
```

### 3. 运行同步

```bash
# 干运行 - 仅列出专辑，不下载
./dizzysync --dry-run

# 开始同步
./dizzysync

# 使用自定义配置文件
./dizzysync -c /path/to/config.toml
```

## 配置选项

### 下载格式

在 `config.toml` 中配置要下载的格式：

```toml
[download]
formats = ["MP3", "FLAC"]  # 可选: "128", "MP3", "FLAC", "gift"
```

- `"128"` - 128kbps MP3 (较小文件)
- `"MP3"` - 320kbps MP3 (高质量)
- `"FLAC"` - 无损FLAC (最高质量)
- `"gift"` - 特典内容

### 文件组织

#### 目录模板

```toml
[paths]
# 自定义目录结构，支持变量替换
directory_template = "{album}/@{label}"  # 默认: 专辑名/@厂牌名
# directory_template = "{year}/{label}/{album}"  # 按年份分类
# directory_template = "{label}/{album}"  # 按厂牌分类
```

**支持的变量：**
- `{album}` - 专辑名
- `{label}` - 厂牌名  
- `{authors}` - 作者名
- `{year}` - 当前年份
- `{date}` - 当前日期 (YYYY-MM-DD)

#### Flatten选项

```toml
[download]
flatten = false  # 控制是否创建格式子文件夹
```

- `false` (默认): 创建格式子文件夹
  ```
  DizzySync/
  ├─ Example Album/
  │  └─ @Example Label/
  │     ├─ MP3/
  │     │  ├─ 01 Track One.mp3
  │     │  └─ 02 Track Two.mp3
  │     └─ FLAC/
  │        ├─ 01 Track One.flac
  │        └─ 02 Track Two.flac
  ```

- `true`: 铺平，不创建格式子文件夹
  ```
  DizzySync/
  └─ Example Album/
     └─ @Example Label/
        ├─ 01 Track One.mp3
        ├─ 02 Track Two.mp3
        ├─ 01 Track One.flac
        └─ 02 Track Two.flac
  ```

## 项目路线图

### Phase 1: Core Demo (当前)
- [x] 配置文件解析
- [x] HTTP客户端和cookie管理
- [x] 用户信息获取
- [x] 专辑列表获取
- [x] 文件下载功能
- [x] 文件组织逻辑
- [x] 命令行界面

### Phase 2: GUI界面 (计划中)
- [ ] Tauri框架集成
- [ ] Web前端界面
- [ ] 下载进度显示
- [ ] 配置管理界面

### Phase 3: 功能增强 (未来)
- [ ] 断点续传
- [ ] 增量同步
- [ ] 多用户支持
- [ ] 自动更新

## 故障排除

### 常见问题

1. **"无法从页面中提取用户ID"**
   - 检查cookie是否正确
   - 确保cookie没有过期

2. **"无法从页面中提取下载密钥"**
   - 某些专辑可能不支持特定格式
   - 尝试其他格式或检查专辑页面

3. **下载失败**
   - 网络连接问题
   - 服务器临时不可用
   - Cookie过期

### 获取帮助

如果遇到问题，请：
1. 检查日志输出
2. 尝试使用 `--dry-run` 模式
3. 提交issue并附上错误信息

## 免责声明

此工具仅用于下载用户已合法购买的内容。请遵守Dizzylab的服务条款和版权法律。

## License

MIT License 