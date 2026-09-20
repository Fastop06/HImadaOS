import sys
with open('hello-linux/src/main.rs', 'r') as f:
    content = f.read()

loop_old = """    let sockfd = syscall3(SYS_SOCKET, 2, 1, 0);"""
loop_new = """    loop {
        let sockfd = syscall3(SYS_SOCKET, 2, 1, 0);"""
content = content.replace(loop_old, loop_new)

end_old = """    syscall1(SYS_CLOSE, connfd);
    syscall1(93, 0);
    
    loop {}"""
end_new = """    syscall1(SYS_CLOSE, connfd);
        // We also need to close sockfd! Wait, connfd == sockfd in my accept hack!
        // But wait, my accept hack returned a NEW fd (i). So we need to close BOTH!
        // No wait, if I loop, I will just leak FDs.
        // Let's close both!
        syscall1(SYS_CLOSE, sockfd);
        for _ in 0..1000000 { core::hint::spin_loop(); }
    }"""
content = content.replace(end_old, end_new)

with open('hello-linux/src/main.rs', 'w') as f:
    f.write(content)
