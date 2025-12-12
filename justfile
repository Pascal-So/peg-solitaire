set dotenv-load

local-server-port := "8081"
bloom-filter := "502115651"


deploy:
    @echo "deploying to $server"

[parallel]
dev: serve-filters dev-frontend

# Run the frontend in development mode on localhost
[working-directory: 'frontend']
dev-frontend:
    BLOOM_FILTER_URL="http://localhost:{{ local-server-port }}/filter_{{ bloom-filter }}_1_norm.bin" trunk serve --release

# Serve the bloom filters on localhost
[working-directory: 'precompute/filters/modulo']
serve-filters:
    npx http-server -p {{ local-server-port }} --cors


test:
    cargo test

[confirm]
clean:
    rm -rf target/
    rm -rf frontend/dist/

