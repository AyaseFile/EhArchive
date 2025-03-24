# EhArchive

将 E-Hentai 画廊归档及翻译后的元数据添加到 calibre 的工具, 使用 Rust 编写

搭配[油猴脚本](./tampermonkey.user.js)使用

需要提供 E-Hentai 账号 cookies, 现存的 calibre 数据库根路径, 和保存标签翻译数据库的路径

支持的 API:
- `/download`: POST, 下载画廊归档, 获取元数据并入库 (calibre)
- `/tasks`: GET, 下载任务状态
- `/import`: POST, 导入**能被后端访问**的归档, 获取元数据并入库 (calibre)

```
Usage: eh-archive [OPTIONS] <ARGUMENTS>

Arguments:
  <IPB_MEMBER_ID>  [env: EH_AUTH_ID=]
  <IPB_PASS_HASH>  [env: EH_AUTH_HASH=]
  [IGNEOUS]        [env: EH_AUTH_IGNEOUS=]
  [SITE]           [env: EH_SITE=] [default: e-hentai.org]

Options:
      --port <PORT>                      [env: PORT=] [default: 3000]
      --archive-output <ARCHIVE_OUTPUT>  [env: ARCHIVE_OUTPUT=]
      --library-root <LIBRARY_ROOT>      [env: CALIBRE_LIBRARY_ROOT=]
      --tag-db-root <TAG_DB_ROOT>        [env: TAG_DB_ROOT=]
      --limit <LIMIT>                    [env: LIMIT=] [default: 5]
  -h, --help                             Print help
```

## Build

查看 `flake.nix` 获取细节

## Thanks

- [EhRust](https://github.com/pboymt/EhRust), GPL-3.0
- [Database](https://github.com/EhTagTranslation/Database), 署名-非商业性使用-相同方式共享 3.0 中国大陆
- [Ehentai_metadata](https://github.com/nonpricklycactus/Ehentai_metadata), GPL-3.0
- [calibre](https://github.com/kovidgoyal/calibre), GPL-3.0
- [citadel](https://github.com/every-day-things/citadel), MIT