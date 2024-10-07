set dotenv-load

run:
    cargo run

web:
    cargo build --target wasm32-unknown-unknown
    cp ./target/wasm32-unknown-unknown/debug/${PROJECT_NAME}.wasm ./

publish:
    cargo build --release --target wasm32-unknown-unknown
    cp ./target/wasm32-unknown-unknown/release/${PROJECT_NAME}.wasm ./
    zip ${PROJECT_NAME} ./index.html ./${PROJECT_NAME}.wasm ./gimp/*
