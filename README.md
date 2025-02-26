# gateway-demo
根目录下执行：
```
cargo build --workspace                    # 构建

RUST_LOG=info cargo run --bin gateway      # 启动网关

curl -v http://localhost:8090/echo/        # 测试echo请求
curl -v http://localhost:8090/docker-echo/ # 测试docker请求
curl -v http://localhost:8090/docker-echo-primes/
```
测试样例：
```
# echo
curl -X POST http://localhost:<port>/echo \
    -H "Content-Type: application/json" \
    -d '{"message": "Hello, Actix!"}'

# docker-echo
curl -X POST http://localhost:<port>/echo/uppercase -d "hello"
curl -X POST http://localhost:8051/echo/primes -d "10017221"
```