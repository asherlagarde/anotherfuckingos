import subprocess

code = """#![no_std]
#![no_main]

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "efiapi" fn efi_main(_image_handle: usize, _system_table: usize) -> usize {
    0
}
"""

with open("/tmp/test_efi.rs", "w") as f:
    f.write(code)

res = subprocess.run([
    "rustc",
    "--target", "x86_64-pc-windows-gnu",
    "--crate-type", "bin",
    "--emit", "obj",
    "-C", "panic=abort",
    "-C", "opt-level=2",
    "/tmp/test_efi.rs",
    "-o", "/tmp/test_efi.o"
], capture_output=True, text=True)

print("rustc status:", res.returncode)
print("rustc err:", res.stderr)

res2 = subprocess.run([
    "clang",
    "-target", "x86_64-unknown-windows",
    "-fuse-ld=lld",
    "-nostdlib",
    "-Wl,-subsystem:efi_application",
    "-Wl,-entry:efi_main",
    "/tmp/test_efi.o",
    "-o", "/tmp/BOOTX64.EFI"
], capture_output=True, text=True)

print("clang status:", res2.returncode)
print("clang err:", res2.stderr)
