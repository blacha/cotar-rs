# Changelog



## [0.3.1](https://github.com/blacha/cotar-rs/compare/cotar-cli-v0.3.0...cotar-cli-v0.3.1) (2023-08-06)


### Bug Fixes

* use a bundled sqlite ([48bc736](https://github.com/blacha/cotar-rs/commit/48bc736d0782dad1447b87e8f519b513263b4bfe))

## [0.3.0](https://github.com/blacha/cotar-rs/compare/cotar-cli-v0.2.0...cotar-cli-v0.3.0) (2023-08-06)


### Features

* **cotar:** move to BufReader ([4116ec5](https://github.com/blacha/cotar-rs/commit/4116ec5462921b15f3eb50c17ae0fb094b270e0c))
* **cotar:** only seek if needed ([9c013e1](https://github.com/blacha/cotar-rs/commit/9c013e17a71edc9d3e9379680ef1861f296ab0f0))
* rename create-index to just index ([97b54d4](https://github.com/blacha/cotar-rs/commit/97b54d4695047cab8b89e2856bf851980e7e3289))

## [0.2.0](https://github.com/blacha/cotar-rs/compare/cotar-cli-v0.1.0...cotar-cli-v0.2.0) (2023-03-01)


### Features

* add --force to overwrite existing cotar ([e6c0a02](https://github.com/blacha/cotar-rs/commit/e6c0a02120f442184bd80ff69a3d991d2ba07b62))
* add a validate command to ensure the tar index contains all the tar's files ([bda0b73](https://github.com/blacha/cotar-rs/commit/bda0b738ea044c8db17cc6cb74fb7cfeab2ca8ee))
* create a cli and impl ([5d01956](https://github.com/blacha/cotar-rs/commit/5d019568ce424a8b26eb48eb52ddea5dc1e2e697))
* improve cli output when converting from mbtiles ([9928580](https://github.com/blacha/cotar-rs/commit/99285804907ee90594644e29f42adb5714408ea7))
* v2 cotar header shrinking index size to 16 bytes from 24 bytes ([57ccb40](https://github.com/blacha/cotar-rs/commit/57ccb4031728e7bccc43797e5fb83d928e7e5b33))
