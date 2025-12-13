set dotenv-load

local-server-port := "8081"
bloom-filter := "502115651"
bloom-filter-filename := "filter_" + bloom-filter + "_1_norm.bin"
server-path := env("SERVER_PATH")


[parallel]
dev: serve-filters dev-frontend

# Run the frontend in development mode on localhost
[working-directory: 'frontend']
dev-frontend:
    BLOOM_FILTER_URL="http://localhost:{{ local-server-port }}/{{ bloom-filter-filename }}" trunk serve --release

# Serve the bloom filters on localhost
[working-directory: 'precompute/filters/modulo']
serve-filters:
    npx http-server -p {{ local-server-port }} --cors

[working-directory: 'frontend']
build-frontend:
    BLOOM_FILTER_URL="{{ bloom-filter-filename }}" trunk --config ./Trunk.deploy.toml build --release --dist ./dist

[working-directory: 'report']
build-report:
    typst compile paper.typ

compress-filter:
    gzip -k -9 {{ bloom-filter-filename }}
    brotli -k -Z {{ bloom-filter-filename }}

deploy: build-frontend build-report
    @echo "deploying to {{ server-path }}"
    rsync -avzi ./frontend/dist/ "{{ server-path }}"
    scp ./report/paper.pdf "{{ server-path / "precomputing-pegsolitaire-paper.pdf" }}"
    rsync -avzi ./precompute/filters/modulo/{{ bloom-filter-filename }}* "{{ server-path }}"
    rsync -avzi .htaccess "{{ server-path }}"

test:
    cargo test

# Remove all cached build artifacts
[confirm]
clean:
    rm -rf target/
    rm -rf frontend/dist/
    rm -rf dist/

