import subprocess
import time

cmd = (
    b"uname\n"
    b"whoami\n"
    b"date\n"
    b"cpuid\n"
    b"lspci\n"
    b"calc 40 + 2\n"
    b"cowsay Look at this OS\n"
    b"fortune\n"
    b"8ball Will this compile?\n"
    b"roast\n"
    b"joke\n"
    b"art skull\n"
    b"run shithole\n"
    b"shutdown\n"
)

p = subprocess.Popen(
    ["qemu-system-x86_64", "-drive", "file=build/os.img,format=raw", "-serial", "stdio", "-display", "none", "-no-reboot", "-m", "256M"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)

try:
    stdout, stderr = p.communicate(input=cmd, timeout=8)
    print("=== QEMU STDOUT ===")
    print(stdout.decode("utf-8", errors="replace"))
    print("=== QEMU STDERR ===")
    print(stderr.decode("utf-8", errors="replace"))
except subprocess.TimeoutExpired:
    p.kill()
    stdout, stderr = p.communicate()
    print("=== QEMU TIMEOUT STDOUT ===")
    print(stdout.decode("utf-8", errors="replace"))
