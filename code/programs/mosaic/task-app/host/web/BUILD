cd ../../../../../packages/typescript/directed-graph && npm ci --quiet
cd ../../../../../packages/typescript/cli-builder && npm ci --quiet
cd ../../../../../packages/typescript/grammar-tools && npm ci --quiet
cd ../../../../../packages/typescript/lexer && npm ci --quiet
cd ../../../../../packages/typescript/state-machine && npm ci --quiet
cd ../../../../../packages/typescript/parser && npm ci --quiet
cd ../../../../../packages/typescript/sql-lexer && npm ci --quiet
cd ../../../../../packages/typescript/sql-parser && npm ci --quiet
cd ../../../../../packages/typescript/sql-execution-engine && npm ci --quiet
cd ../../../../../packages/typescript/storage && npm ci --quiet
cd ../../../../../packages/typescript/indexeddb && npm ci --quiet
npm ci --quiet
rustup target add wasm32-unknown-unknown
cargo build --manifest-path ../../../../../packages/rust/Cargo.toml -p task-wasm --target wasm32-unknown-unknown --release
npx vitest run
npm run build
