wpmeta
======

Wallpaper metadata generator

Usage
-----

```
WPMETA_LOG=info cargo run -p -- --src <SRCDIR> --dst <PKGDIR>
```

Example Metadata
----------------

```toml
[[authors]]
 email = "yajuu.senpai@example.com"
name.default = "Yajuu Senpai"
name.zh-CN = "野兽先辈"

[[wallpapers]]
title.default = "Kusa"
title.en-US = "Grass"
license = "CC BY-SA 4.0"
id = "Kusa"
path = "kusa.jpg"
```
