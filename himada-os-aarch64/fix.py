with open("src/main.rs", "r") as f:
    lines = f.readlines()
lines = [l for l in lines if not l.strip().startswith("#![no_")]
lines = [l for l in lines if not l.strip().startswith("#![feature")]
with open("src/main.rs", "w") as f:
    f.write("#![no_std]\n#![no_main]\n#![feature(naked_functions)]\n" + "".join(lines))
