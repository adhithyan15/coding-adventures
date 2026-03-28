# Install transitive file: dependencies in leaf-to-root order.
# uuid depends on sha1 and md5 (both are leaves with no file: deps).
# directed-graph has no file: deps.
# CI starts with clean node_modules everywhere, so each package must be
# installed before any package that depends on it.
cd ../../../packages/typescript/sha1 && npm install --quiet && \
cd ../md5 && npm install --quiet && \
cd ../uuid && npm install --quiet && \
cd ../directed-graph && npm install --quiet && \
cd ../../../programs/typescript/todo-app && npm install --quiet && \
npx vitest run
