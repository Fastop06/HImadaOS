import sys
with open('hello-linux/src/main.rs', 'r') as f:
    content = f.read()

read_old = """    let mut buf = [0u8; 1024];
    let _n = syscall3(SYS_READ, connfd, buf.as_mut_ptr() as usize, 1024);"""
read_new = """    let mut buf = [0u8; 1024];
    let mut _n = !0;
    while _n == !0 {
        _n = syscall3(SYS_READ, connfd, buf.as_mut_ptr() as usize, 1024);
        for _ in 0..10000 { core::hint::spin_loop(); }
    }"""
content = content.replace(read_old, read_new)

with open('hello-linux/src/main.rs', 'w') as f:
    f.write(content)
