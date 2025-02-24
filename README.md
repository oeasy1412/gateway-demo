# gateway-demo
根目录下执行：
```
cargo build --workspace  # 构建

RUST_LOG=info cargo run --bin gateway  # 启动网关

curl -v http://localhost:8090/echo/ # 测试请求
```