#!/usr/bin/env python3
import os
import sys
import subprocess
import glob

def run_cmd(cmd, cwd=None):
    print(f"[RUN] {' '.join(cmd) if isinstance(cmd, list) else cmd}")
    res = subprocess.run(cmd, cwd=cwd, shell=isinstance(cmd, str))
    if res.returncode != 0:
        print(f"[ERROR] Command failed with exit code {res.returncode}")
        sys.exit(res.returncode)

def main():
    os.makedirs("build", exist_ok=True)

    print(">>> 1. Compiling Linux userland ELF binaries with rustc...")
    run_cmd([
        "rustc",
        "--target", "x86_64-unknown-linux-gnu",
        "-C", "panic=abort",
        "-C", "opt-level=2",
        "-C", "relocation-model=static",
        "-C", "link-args=-nostdlib -static",
        "userland/hello_linux.rs",
        "-o", "build/hello_linux.elf"
    ])

    run_cmd([
        "rustc",
        "--target", "x86_64-unknown-linux-gnu",
        "-C", "panic=abort",
        "-C", "opt-level=2",
        "-C", "relocation-model=static",
        "-C", "link-args=-nostdlib -static",
        "userland/counter.rs",
        "-o", "build/counter.elf"
    ])

    run_cmd([
        "rustc",
        "--target", "x86_64-unknown-linux-gnu",
        "-C", "panic=abort",
        "-C", "opt-level=2",
        "-C", "relocation-model=static",
        "-C", "link-args=-nostdlib -static",
        "userland/shithole.rs",
        "-o", "build/shithole.elf"
    ])

    run_cmd(["strip", "--strip-all", "build/hello_linux.elf"])
    run_cmd(["strip", "--strip-all", "build/counter.elf"])
    run_cmd(["strip", "--strip-all", "build/shithole.elf"])

    print(">>> 2. Assembling bootloader (boot.asm)...")
    run_cmd(["nasm", "-f", "bin", "boot/boot.asm", "-o", "build/boot.bin"])

    print(">>> 3. Assembling 64-bit stage 2 entry (entry64.asm)...")
    run_cmd(["nasm", "-f", "elf64", "boot/entry64.asm", "-o", "build/entry64.o"])

    print(">>> 4. Assembling syscall assembly (syscall_asm.asm)...")
    run_cmd(["nasm", "-f", "elf64", "kernel/src/syscall_asm.asm", "-o", "build/syscall_asm.o"])

    print(">>> 5. Assembling IDT exception stubs (idt_asm.asm)...")
    run_cmd(["nasm", "-f", "elf64", "kernel/src/idt_asm.asm", "-o", "build/idt_asm.o"])

    print(">>> 6. Compiling 64-bit Rust Kernel (kernel/src/main.rs)...")
    run_cmd([
        "rustc",
        "--crate-type", "lib",
        "--target", "x86_64-unknown-linux-gnu",
        "-C", "panic=abort",
        "-C", "opt-level=2",
        "-C", "relocation-model=static",
        "-C", "code-model=small",
        "kernel/src/main.rs",
        "-o", "build/librustkernel.a"
    ])

    print(">>> 7. Linking full 64-bit kernel image (entry64.o + assembly + rust kernel)...")
    libcore_files = glob.glob("/usr/lib/rustlib/x86_64-unknown-linux-gnu/lib/libcore-*.rlib")
    libcore_path = libcore_files[0] if libcore_files else ""
    libbuiltins_files = glob.glob("/usr/lib/rustlib/x86_64-unknown-linux-gnu/lib/libcompiler_builtins-*.rlib")
    libbuiltins_path = libbuiltins_files[0] if libbuiltins_files else ""

    run_cmd([
        "ld",
        "-m", "elf_x86_64",
        "-T", "kernel/linker.ld",
        "--oformat", "binary",
        "build/entry64.o",
        "build/syscall_asm.o",
        "build/idt_asm.o",
        "build/librustkernel.a",
        libcore_path,
        libbuiltins_path,
        "-o", "build/kernel.bin"
    ])

    print(">>> 8. Creating bootable hard disk image (build/os.img)...")
    with open("build/boot.bin", "rb") as f:
        boot_bin = f.read()
    if len(boot_bin) != 512:
        print(f"[ERROR] Boot sector size is {len(boot_bin)} bytes, must be 512!")
        sys.exit(1)

    with open("build/kernel.bin", "rb") as f:
        kernel_bin = f.read()

    print(f"[INFO] Boot sector: {len(boot_bin)} bytes, Kernel binary: {len(kernel_bin)} bytes")

    # 10 MB raw hard disk image
    total_size = 10 * 1024 * 1024
    full_image = boot_bin + kernel_bin
    if len(full_image) < total_size:
        full_image += b'\x00' * (total_size - len(full_image))

    with open("build/os.img", "wb") as f:
        f.write(full_image)

    print(">>> Build successful! Output: build/os.img (10 MB hard disk image)")

if __name__ == "__main__":
    main()
