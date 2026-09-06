# EhArchive

将 E-Hentai 画廊归档及翻译后的元数据打包为 Komga One-Shots CBZ 的工具, 使用 Rust 编写

搭配[油猴脚本](./tampermonkey.user.js)使用

需要提供 E-Hentai 账号 cookies, 归档输出路径, 和保存标签翻译数据库的路径

归档保存为 `<gid>_<token>_<flag>.cbz`, 所有条目均使用 ZIP STORE, `ComicInfo.xml` 位于 CBZ 内的根目录. 原始 gallery metadata 保存为 `<metadata-output>/<gid>_<token>.json`

支持的 API:
- `/downloads`: POST, 下载画廊归档并写入元数据
- `/tasks`: GET, 下载任务状态
- `/imports`: POST, 导入**能被后端访问**的归档并写入元数据

```
Usage: eh-archive [OPTIONS] --archive-output <ARCHIVE_OUTPUT> --tag-db-root <TAG_DB_ROOT> <IPB_MEMBER_ID> <IPB_PASS_HASH> [IGNEOUS] [SITE]

Arguments:
  <IPB_MEMBER_ID>  [env: EH_AUTH_ID=]
  <IPB_PASS_HASH>  [env: EH_AUTH_HASH=]
  [IGNEOUS]        [env: EH_AUTH_IGNEOUS=]
  [SITE]           [env: EH_SITE=] [default: e-hentai.org]

Options:
      --port <PORT>                          [env: PORT=] [default: 3000]
      --archive-output <ARCHIVE_OUTPUT>      [env: ARCHIVE_OUTPUT=]
      --metadata-output <METADATA_OUTPUT>    [env: METADATA_OUTPUT=]
      --tag-db-root <TAG_DB_ROOT>            [env: TAG_DB_ROOT=]
      --limit <LIMIT>                        [env: LIMIT=] [default: 5]
      --komga-url <KOMGA_URL>                [env: KOMGA_URL=]
      --komga-library-id <KOMGA_LIBRARY_ID>  [env: KOMGA_LIBRARY_ID=]
      --komga-api-key <KOMGA_API_KEY>        [env: KOMGA_API_KEY=]
  -h, --help                                 Print help
```

## Build

查看 `flake.nix` 获取细节

## Thanks

- [EhRust](https://github.com/pboymt/EhRust), GPL-3.0
- [Database](https://github.com/EhTagTranslation/Database), 署名-非商业性使用-相同方式共享 3.0 中国大陆
- [Ehentai_metadata](https://github.com/nonpricklycactus/Ehentai_metadata), GPL-3.0
- [citadel](https://github.com/every-day-things/citadel), MIT