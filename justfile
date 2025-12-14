set dotenv-load

local-server-port := "8081"
bloom-filter := "502115651" # 268435456
bloom-filter-filename := "filter_" + bloom-filter + "_1_norm.bin"
bloom-filter-url := "bloom-filters" / bloom-filter-filename

server-path := env("SERVER_PATH")


# Run the frontend in development mode on localhost
[working-directory: 'frontend']
dev:
    BLOOM_FILTER_URL="{{ bloom-filter-url }}" trunk serve --release

[working-directory: 'frontend']
build-frontend:
    BLOOM_FILTER_URL="{{ bloom-filter-url }}" trunk --config ./Trunk.deploy.toml build --release --dist ./dist

[working-directory: 'report']
build-report:
    typst compile paper.typ

[working-directory: 'report']
watch-report:
    typst watch paper.typ

[working-directory: 'frontend/bloom-filters']
compress-filter:
    gzip -k -9 {{ bloom-filter-filename }}
    brotli -k -Z {{ bloom-filter-filename }}

# Build the application and upload it to the webserver
deploy: build-frontend build-report
    @echo "deploying to {{ server-path }}"
    rsync -avzi ./frontend/dist/ "{{ server-path }}"
    scp ./report/paper.pdf "{{ server-path / "precomputing-pegsolitaire-paper.pdf" }}"
    rsync -avzi .htaccess "{{ server-path }}"

test:
    cargo test

# Remove all cached build artifacts
[confirm]
clean:
    rm -rf target/
    rm -rf frontend/dist/
    rm -rf dist/

