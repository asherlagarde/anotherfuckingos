import subprocess

p = subprocess.Popen(
    ["qemu-system-x86_64", "-drive", "file=build/os.img,format=raw", "-serial", "stdio", "-display", "none", "-no-reboot", "-m", "256M"],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True
)

try:
    stdout, stderr = p.communicate(timeout=4)
    print("STDOUT:", repr(stdout))
    print("STDERR:", repr(stderr))
except subprocess.TimeoutExpired:
    p.kill()
    stdout, stderr = p.communicate()
    print("TIMEOUT STDOUT:", repr(stdout))
    print("TIMEOUT STDERR:", repr(stderr))
