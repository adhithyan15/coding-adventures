cd ../../../../../packages/typescript/directed-graph && npm install --quiet
cd ../../../../../packages/typescript/cli-builder && npm install --quiet
cd ../../../../../packages/typescript/grammar-tools && npm install --quiet
cd ../../../../../packages/typescript/lexer && npm install --quiet
cd ../../../../../packages/typescript/state-machine && npm install --quiet
cd ../../../../../packages/typescript/parser && npm install --quiet
cd ../../../../../packages/typescript/sql-lexer && npm install --quiet
cd ../../../../../packages/typescript/sql-parser && npm install --quiet
cd ../../../../../packages/typescript/sql-execution-engine && npm install --quiet
cd ../../../../../packages/typescript/storage && npm install --quiet
cd ../../../../../packages/typescript/indexeddb && npm install --quiet
npm install --quiet
rustup target add wasm32-unknown-unknown
cargo build --manifest-path ../../../../../packages/rust/Cargo.toml -p task-wasm --target wasm32-unknown-unknown --release
npx vitest run
npm run build
