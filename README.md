<h2>Environment Variables</h2>

Only required for testing

```conf
TWELVE_API_KEY=YOUR_TWELVE_API_KEY
```

<h2>Build to WASM (For Website)</h2>

Ensure 'crate-type = ["cdylib"]' is added under '[lib]' in the library Cargo.toml workspace.

!!! Comment this out again once done - otherwise you will receive issues on importing this as a library !!!

IMPORTANT: The Twelve API Key is stored in both the .env but also hard coded in prelude.rs. HIDE THIS ON YOUR OWN SERVER ONCE SET UP! All Twelve calls should go through a Web Server.

```shell
cargo build
wasm-pack build --target web
```

if you want to debug issues, you can use:

```shell
cargo build
wasm-pack build --target web --dev
```

Notice the pkg folder. This will contain what you need for the Javascript project.

Or if you just want the wasm binary in the target folder:

```shell
cargo build
cargo build --target wasm32-unknown-unknown --release
```
