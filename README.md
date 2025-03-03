# gateway-demo
根目录下执行：
```
cargo build --workspace                    # 构建

RUST_LOG=info cargo run --bin gateway      # 启动网关

curl -v http://localhost:8090/echo        # 测试echo请求
curl -v http://localhost:8090/docker-echo # 测试docker请求
curl -v http://localhost:8090/docker-echo-primes
```
测试样例：
```
# echo
<<<<<<< HEAD

curl -v  POST http://localhost:8090/echo/echo \
=======
curl -v -X POST http://localhost:8090/echo/echo \
>>>>>>> 76bd215 (基本解耦了一下，将功能从main内部拆分出去)
    -H "Content-Type: application/json" \
    -d '{"message": "Hello, Actix!"}'

# docker-echo

<<<<<<< HEAD
=======
curl -v -X POST http://localhost:8090/echo  -d "hello"
>>>>>>> 76bd215 (基本解耦了一下，将功能从main内部拆分出去)
```