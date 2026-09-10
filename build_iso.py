#!/usr/bin/env python3
"""
build_iso.py - AnotherFuckingOs ISO Builder
Creates a hybrid BIOS + UEFI bootable ISO image.

This script:
  1. First runs build.py to compile the kernel and create os.img
  2. Creates a BIOS El Torito boot image from the MBR bootloader + kernel
  3. Creates an EFI System Partition FAT image (for UEFI boot)
  4. Packages everything into a hybrid bootable ISO with xorriso

Requirements (install via apt):
  sudo apt-get install -y xorriso mtools dosfstools
"""

import os
import sys
import struct
import subprocess
import shutil
import tempfile

PROJECT_DIR = os.path.dirname(os.path.abspath(__file__))
BUILD_DIR = os.path.join(PROJECT_DIR, "build")
ISO_DIR = os.path.join(BUILD_DIR, "iso_root")
OUTPUT_ISO = os.path.join(BUILD_DIR, "AnotherFuckingOs.iso")


def run_cmd(cmd, cwd=None, check=True):
    """Run a shell command and print it."""
    cmd_str = ' '.join(cmd) if isinstance(cmd, list) else cmd
    print(f"  [RUN] {cmd_str}")
    result = subprocess.run(
        cmd, cwd=cwd, shell=isinstance(cmd, str),
        capture_output=True, text=True
    )
    if result.stdout.strip():
        for line in result.stdout.strip().split('\n')[:10]:
            print(f"        {line}")
    if result.returncode != 0 and check:
        print(f"  [ERROR] Command failed (exit {result.returncode})")
        if result.stderr.strip():
            for line in result.stderr.strip().split('\n')[:10]:
                print(f"        {line}")
        sys.exit(result.returncode)
    return result


def check_tools():
    """Check that required tools are available."""
    print("\n>>> 0. Checking required tools...")
    tools = {
        "xorriso": "sudo apt-get install -y xorriso",
        "mcopy": "sudo apt-get install -y mtools",
        "mkfs.fat": "sudo apt-get install -y dosfstools",
    }
    missing = []
    for tool, install in tools.items():
        result = subprocess.run(["which", tool], capture_output=True)
        if result.returncode != 0:
            missing.append((tool, install))
            print(f"  [MISSING] {tool} - install with: {install}")

    if missing:
        print("\n  [INFO] Attempting to install missing tools...")
        packages = set()
        for _, install_cmd in missing:
            # Extract package name from install command
            pkg = install_cmd.split()[-1]
            packages.add(pkg)
        run_cmd(["sudo", "apt-get", "install", "-y"] + list(packages))

        # Re-check
        for tool, install in missing:
            result = subprocess.run(["which", tool], capture_output=True)
            if result.returncode != 0:
                print(f"  [FATAL] Still can't find {tool} after install attempt!")
                sys.exit(1)

    print("  [OK] All tools available.")


def build_kernel():
    """Run the main build.py to compile kernel and create os.img."""
    print("\n>>> 1. Building kernel (running build.py)...")
    build_script = os.path.join(PROJECT_DIR, "build.py")
    if not os.path.exists(build_script):
        print("  [ERROR] build.py not found!")
        sys.exit(1)
    run_cmd(["python3", build_script], cwd=PROJECT_DIR)
    
    # Verify outputs
    os_img = os.path.join(BUILD_DIR, "os.img")
    kernel_bin = os.path.join(BUILD_DIR, "kernel.bin")
    boot_bin = os.path.join(BUILD_DIR, "boot.bin")
    
    for f in [os_img, kernel_bin, boot_bin]:
        if not os.path.exists(f):
            print(f"  [ERROR] Expected build output missing: {f}")
            sys.exit(1)
    
    boot_size = os.path.getsize(boot_bin)
    kernel_size = os.path.getsize(kernel_bin)
    img_size = os.path.getsize(os_img)
    print(f"  [OK] boot.bin: {boot_size} bytes, kernel.bin: {kernel_size} bytes, os.img: {img_size} bytes")


def create_bios_boot_image():
    """
    Create a BIOS El Torito boot image.
    
    For El Torito "no emulation" mode, we need a flat binary that the BIOS
    loads and executes. We'll create a boot image that contains:
    - The MBR bootloader (512 bytes) modified to load from CD-ROM
    - The kernel binary immediately following
    
    But actually, the simplest approach for our MBR-based OS is to use
    El Torito "hard disk emulation" mode, which makes the BIOS treat our
    os.img as if it were a hard drive - our bootloader works unchanged!
    
    Even simpler: use xorriso's -isohybrid-mbr to embed the MBR directly,
    making the ISO itself look like a bootable disk to BIOS.
    """
    print("\n>>> 2. Preparing BIOS boot image...")
    
    # For El Torito hard disk emulation, we use a small boot catalog image.
    # But the most reliable method is hybrid ISO: embed MBR boot code directly
    # into the ISO's System Area (first 32KB) using isohybrid/xorriso.
    #
    # We'll create a 2880KB (1.44MB floppy) El Torito image for max compatibility.
    # This floppy image contains: boot sector + kernel (up to ~1.44MB).
    
    boot_bin = os.path.join(BUILD_DIR, "boot.bin")
    kernel_bin = os.path.join(BUILD_DIR, "kernel.bin")
    eltorito_img = os.path.join(BUILD_DIR, "eltorito.img")
    
    with open(boot_bin, "rb") as f:
        boot_data = f.read()
    with open(kernel_bin, "rb") as f:
        kernel_data = f.read()
    
    # Create a raw image: boot sector + kernel, padded to fill usable space
    raw = boot_data + kernel_data
    
    # Determine image size: use 2880 sectors (1.44MB floppy) if it fits,
    # otherwise use hard disk emulation with a larger image
    floppy_size = 2880 * 512  # 1.44MB
    
    if len(raw) <= floppy_size:
        # Fits in a 1.44MB floppy image - best BIOS compatibility
        img_size = floppy_size
        emul_type = "floppy"
        print(f"  [INFO] Kernel fits in 1.44MB floppy image ({len(raw)} bytes)")
    else:
        # Too big for floppy, use no-emulation mode with custom loader
        # Round up to 2048-byte CD sectors
        cd_sectors = (len(raw) + 2047) // 2048
        img_size = cd_sectors * 2048
        emul_type = "no-emulation"
        print(f"  [INFO] Kernel too large for floppy ({len(raw)} bytes), using no-emulation mode")
    
    img_data = raw + b'\x00' * (img_size - len(raw))
    
    with open(eltorito_img, "wb") as f:
        f.write(img_data)
    
    print(f"  [OK] BIOS boot image: {eltorito_img} ({len(img_data)} bytes, {emul_type})")
    return eltorito_img, emul_type


def create_efi_boot_image():
    """
    Create an EFI System Partition (ESP) FAT image containing BOOTX64.EFI.
    
    The EFI bootloader (boot/efi/) needs to be compiled as a PE32+ binary.
    If we can't compile it, we'll skip EFI boot support.
    """
    print("\n>>> 3. Creating EFI boot image...")
    
    efi_img = os.path.join(BUILD_DIR, "efi.img")
    efi_boot_dir = os.path.join(BUILD_DIR, "efi_staging")
    
    # Check if we can build the EFI bootloader
    efi_src = os.path.join(PROJECT_DIR, "boot", "efi")
    efi_binary = None
    
    # Try to compile the UEFI bootloader with cargo + rustup
    try:
        cargo_result = subprocess.run(
            ["which", "cargo"], capture_output=True
        )
        
        if cargo_result.returncode == 0 and os.path.exists(efi_src):
            print("  [INFO] Attempting to compile UEFI bootloader...")
            
            # Check if rustup is available for target management
            try:
                target_check = subprocess.run(
                    ["rustup", "target", "list", "--installed"],
                    capture_output=True, text=True
                )
                has_uefi_target = "x86_64-unknown-uefi" in (target_check.stdout if target_check.returncode == 0 else "")
                
                if not has_uefi_target:
                    print("  [INFO] Adding x86_64-unknown-uefi target...")
                    run_cmd(["rustup", "target", "add", "x86_64-unknown-uefi"], check=False)
            except FileNotFoundError:
                print("  [INFO] rustup not found, skipping target check...")
            
            # Try building the EFI bootloader
            build_result = run_cmd([
                "cargo", "build",
                "--release",
                "--target", "x86_64-unknown-uefi",
                "--manifest-path", os.path.join(efi_src, "Cargo.toml"),
            ], check=False)
            
            # Look for the .efi binary
            efi_binary_path = os.path.join(
                efi_src, "target", "x86_64-unknown-uefi", "release", "rustos-efi-boot.efi"
            )
            if os.path.exists(efi_binary_path):
                efi_binary = efi_binary_path
                print(f"  [OK] UEFI bootloader compiled: {efi_binary_path}")
            else:
                # Check alternative names
                for name in ["rustos_efi_boot.efi", "rustos-efi-boot.efi"]:
                    alt = os.path.join(efi_src, "target", "x86_64-unknown-uefi", "release", name)
                    if os.path.exists(alt):
                        efi_binary = alt
                        print(f"  [OK] UEFI bootloader compiled: {alt}")
                        break
    except FileNotFoundError:
        print("  [INFO] cargo not found, skipping UEFI bootloader compilation...")
    
    if efi_binary is None:
        print("  [WARN] Could not compile UEFI bootloader. Creating stub EFI image...")
        print("         The ISO will boot via BIOS only.")
        # Create a minimal stub EFI application that prints a message
        # This is a valid PE32+ that just halts
        efi_binary = create_stub_efi(BUILD_DIR)
    
    # Create FAT12 filesystem image for EFI System Partition
    # Size: 1.44MB (standard EFI boot floppy) or larger
    efi_binary_size = os.path.getsize(efi_binary)
    # Need at least: FAT overhead (~32KB) + EFI binary + padding
    min_size = max(1440 * 1024, efi_binary_size + 64 * 1024)
    # Round up to nearest MB
    esp_size_kb = ((min_size + 1023) // 1024)
    # Minimum 2880 sectors for mkfs.fat to be happy
    esp_size_kb = max(esp_size_kb, 2880)
    
    # Create empty FAT image
    run_cmd(["dd", "if=/dev/zero", f"of={efi_img}", "bs=1K", f"count={esp_size_kb}"])
    
    # Format as FAT12/FAT16
    run_cmd(["mkfs.fat", "-F", "12", efi_img])
    
    # Create EFI/BOOT directory structure and copy BOOTX64.EFI using mtools
    run_cmd(["mmd", "-i", efi_img, "::EFI"])
    run_cmd(["mmd", "-i", efi_img, "::EFI/BOOT"])
    run_cmd(["mcopy", "-i", efi_img, efi_binary, "::EFI/BOOT/BOOTX64.EFI"])
    
    print(f"  [OK] EFI System Partition image: {efi_img} ({esp_size_kb}KB)")
    return efi_img


def create_stub_efi(build_dir):
    """Create a minimal stub UEFI PE32+ application that just halts."""
    stub_path = os.path.join(build_dir, "BOOTX64.EFI")
    
    # Minimal valid PE32+ for x86_64 UEFI that does nothing (returns)
    # This is a hand-crafted minimal PE header + a single RET instruction
    
    # DOS Header
    dos_header = bytearray(64)
    dos_header[0:2] = b'MZ'  # e_magic
    struct.pack_into('<I', dos_header, 60, 64)  # e_lfanew -> PE header at offset 64
    
    # PE Signature
    pe_sig = b'PE\x00\x00'
    
    # COFF Header (20 bytes)
    coff = bytearray(20)
    struct.pack_into('<H', coff, 0, 0x8664)  # Machine: AMD64
    struct.pack_into('<H', coff, 2, 1)        # NumberOfSections: 1
    struct.pack_into('<H', coff, 16, 112)     # SizeOfOptionalHeader
    struct.pack_into('<H', coff, 18, 0x0022)  # Characteristics: EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE
    
    # Optional Header (PE32+)
    opt = bytearray(112)
    struct.pack_into('<H', opt, 0, 0x020B)    # Magic: PE32+
    struct.pack_into('<I', opt, 16, 0x1000)   # AddressOfEntryPoint
    struct.pack_into('<Q', opt, 24, 0x10000)  # ImageBase
    struct.pack_into('<I', opt, 32, 0x1000)   # SectionAlignment
    struct.pack_into('<I', opt, 36, 0x200)    # FileAlignment
    struct.pack_into('<H', opt, 40, 1)        # MajorOSVersion
    struct.pack_into('<I', opt, 56, 0x3000)   # SizeOfImage
    struct.pack_into('<I', opt, 60, 0x200)    # SizeOfHeaders
    struct.pack_into('<H', opt, 68, 10)       # Subsystem: EFI Application
    struct.pack_into('<I', opt, 84, 0x1000)   # SizeOfStackReserve (low)
    struct.pack_into('<I', opt, 92, 0x1000)   # SizeOfStackCommit (low)
    struct.pack_into('<I', opt, 108, 0)       # NumberOfRvaAndSizes
    
    # Section Header (.text)
    sect = bytearray(40)
    sect[0:6] = b'.text\x00'
    struct.pack_into('<I', sect, 8, 0x10)     # VirtualSize
    struct.pack_into('<I', sect, 12, 0x1000)  # VirtualAddress
    struct.pack_into('<I', sect, 16, 0x200)   # SizeOfRawData
    struct.pack_into('<I', sect, 20, 0x200)   # PointerToRawData
    struct.pack_into('<I', sect, 36, 0x60000020)  # Characteristics: code|exec|read
    
    # Code section: xor eax, eax; ret (EFI_SUCCESS = 0)
    code = bytearray(0x200)
    code[0] = 0x31  # xor eax, eax
    code[1] = 0xC0
    code[2] = 0xC3  # ret
    
    # Assemble the PE file
    pe_data = bytearray(0x200)  # Headers (file-aligned to 0x200)
    pe_data[0:len(dos_header)] = dos_header
    offset = 64
    pe_data[offset:offset+4] = pe_sig
    offset += 4
    pe_data[offset:offset+20] = coff
    offset += 20
    pe_data[offset:offset+112] = opt
    offset += 112
    pe_data[offset:offset+40] = sect
    
    # Pad headers to file alignment
    while len(pe_data) < 0x200:
        pe_data.append(0)
    
    # Append code section
    pe_data += code
    
    # Pad to fill SizeOfImage alignment
    while len(pe_data) < 0x3000:
        pe_data.append(0)
    
    with open(stub_path, "wb") as f:
        f.write(pe_data)
    
    print(f"  [INFO] Created stub BOOTX64.EFI ({len(pe_data)} bytes)")
    return stub_path


def create_iso_root():
    """Create the ISO filesystem root directory."""
    print("\n>>> 4. Creating ISO filesystem root...")
    
    if os.path.exists(ISO_DIR):
        shutil.rmtree(ISO_DIR)
    os.makedirs(ISO_DIR)
    
    # Create directory structure
    os.makedirs(os.path.join(ISO_DIR, "boot"), exist_ok=True)
    os.makedirs(os.path.join(ISO_DIR, "boot", "grub"), exist_ok=True)
    os.makedirs(os.path.join(ISO_DIR, "AnotherFuckingOs"), exist_ok=True)
    
    # Copy kernel binary and ELF binaries into the ISO
    shutil.copy2(
        os.path.join(BUILD_DIR, "kernel.bin"),
        os.path.join(ISO_DIR, "AnotherFuckingOs", "kernel.bin")
    )
    
    for elf in ["hello_linux.elf", "counter.elf", "shithole.elf"]:
        elf_path = os.path.join(BUILD_DIR, elf)
        if os.path.exists(elf_path):
            shutil.copy2(elf_path, os.path.join(ISO_DIR, "AnotherFuckingOs", elf))
    
    # Create a README.txt on the ISO
    readme = os.path.join(ISO_DIR, "README.TXT")
    with open(readme, "w") as f:
        f.write("=" * 60 + "\n")
        f.write("  AnotherFuckingOs v0.1 - Because Why Not\n")
        f.write("=" * 60 + "\n\n")
        f.write("You actually downloaded this. Incredible.\n\n")
        f.write("BOOT INSTRUCTIONS:\n")
        f.write("  BIOS:  Just boot from this disc. It works. Probably.\n")
        f.write("  UEFI:  Boot from the EFI partition. Same thing, fancier.\n")
        f.write("  QEMU:  qemu-system-x86_64 -cdrom AnotherFuckingOs.iso\n\n")
        f.write("FEATURES:\n")
        f.write("  - 70+ commands, all brutally honest\n")
        f.write("  - Linux ABI compatible (lol)\n")
        f.write("  - ELF binary execution\n")
        f.write("  - Hardware diagnostics that roast your setup\n")
        f.write("  - Games that make you question your life choices\n\n")
        f.write("SYSTEM REQUIREMENTS:\n")
        f.write("  - An x86_64 CPU (or QEMU, because you're not insane)\n")
        f.write("  - 512KB RAM (we're not Chrome)\n")
        f.write("  - A sense of humor (critical dependency)\n")
        f.write("  - Low expectations (recommended)\n\n")
        f.write("LICENSE: WTFPL - Do What The Fuck You Want Public License\n")
        f.write("SUPPORT: lmao\n")
    
    print(f"  [OK] ISO root created at {ISO_DIR}")


def build_iso(eltorito_img, emul_type, efi_img):
    """
    Build the final ISO using xorriso.
    
    Creates a hybrid bootable ISO with:
    - BIOS boot via El Torito
    - UEFI boot via EFI System Partition  
    - Isohybrid MBR for direct USB boot
    """
    print("\n>>> 5. Building ISO with xorriso...")
    
    # Copy boot images into ISO root
    shutil.copy2(eltorito_img, os.path.join(ISO_DIR, "boot", "eltorito.img"))
    shutil.copy2(efi_img, os.path.join(ISO_DIR, "boot", "efi.img"))
    
    # Also copy the full os.img for reference/alternate boot
    shutil.copy2(
        os.path.join(BUILD_DIR, "os.img"),
        os.path.join(ISO_DIR, "AnotherFuckingOs", "os.img")
    )
    
    # Build ISO with xorriso
    # -as mkisofs: compatibility mode with mkisofs/genisoimage
    # -R: Rock Ridge extensions (Unix file permissions/names)
    # -J: Joliet extensions (Windows long filenames)
    # -V: Volume ID
    # -b: El Torito boot image
    # -no-emul-boot / -boot-load-size: for no-emulation mode
    # -eltorito-alt-boot + -e: EFI boot image
    # -isohybrid-mbr: embed MBR for USB boot
    
    xorriso_cmd = [
        "xorriso", "-as", "mkisofs",
        "-R", "-J",
        "-V", "ANOTHERFUCKINGOS",
        "-A", "AnotherFuckingOs v0.1",
        "-publisher", "Some Lunatic With A Keyboard",
        "-p", "AnotherFuckingOs Build System",
    ]
    
    if emul_type == "floppy":
        # Floppy emulation mode - BIOS loads this as if it's a floppy
        xorriso_cmd += [
            "-b", "boot/eltorito.img",
            "-boot-load-size", "4",  # Load 4 sectors (2KB) for initial bootstrap
            "-boot-info-table",
        ]
    else:
        # No-emulation mode - BIOS loads the specified sectors  
        boot_sectors = os.path.getsize(eltorito_img) // 512
        xorriso_cmd += [
            "-b", "boot/eltorito.img",
            "-no-emul-boot",
            "-boot-load-size", str(boot_sectors),
            "-boot-info-table",
        ]
    
    # Add EFI boot
    xorriso_cmd += [
        "-eltorito-alt-boot",
        "-e", "boot/efi.img",
        "-no-emul-boot",
    ]
    
    # Try isohybrid for USB boot (may need isohdpfx.bin from syslinux)
    isohdpfx = "/usr/lib/ISOLINUX/isohdpfx.bin"
    if os.path.exists(isohdpfx):
        xorriso_cmd += ["-isohybrid-mbr", isohdpfx]
        print("  [INFO] Using ISOLINUX isohdpfx.bin for hybrid MBR")
    else:
        # Try alternate location
        alt_isohdpfx = "/usr/share/syslinux/isohdpfx.bin"
        if os.path.exists(alt_isohdpfx):
            xorriso_cmd += ["-isohybrid-mbr", alt_isohdpfx]
            print("  [INFO] Using syslinux isohdpfx.bin for hybrid MBR")
        else:
            print("  [WARN] No isohdpfx.bin found - ISO won't be USB-bootable (CD/DVD only)")
            print("         Install syslinux/isolinux for USB boot support.")
    
    # Output file and source directory
    xorriso_cmd += [
        "-o", OUTPUT_ISO,
        ISO_DIR,
    ]
    
    run_cmd(xorriso_cmd)
    
    if os.path.exists(OUTPUT_ISO):
        iso_size = os.path.getsize(OUTPUT_ISO)
        iso_mb = iso_size / (1024 * 1024)
        print(f"\n  [OK] ISO created: {OUTPUT_ISO}")
        print(f"       Size: {iso_size} bytes ({iso_mb:.1f} MB)")
    else:
        print(f"\n  [ERROR] ISO file was not created!")
        sys.exit(1)


def print_banner():
    """Print a beautiful banner because we're classy like that."""
    print(r"""
    ╔══════════════════════════════════════════════════════════╗
    ║                                                          ║
    ║    █████╗ ███╗   ██╗ ██████╗ ████████╗██╗  ██╗███████╗  ║
    ║   ██╔══██╗████╗  ██║██╔═══██╗╚══██╔══╝██║  ██║██╔════╝  ║
    ║   ███████║██╔██╗ ██║██║   ██║   ██║   ███████║█████╗    ║
    ║   ██╔══██║██║╚██╗██║██║   ██║   ██║   ██╔══██║██╔══╝    ║
    ║   ██║  ██║██║ ╚████║╚██████╔╝   ██║   ██║  ██║███████╗  ║
    ║   ╚═╝  ╚═╝╚═╝  ╚═══╝ ╚═════╝    ╚═╝   ╚═╝  ╚═╝╚══════╝  ║
    ║   ██████╗ ███████╗      ISO BUILDER                      ║
    ║   ██╔═══██╗██╔════╝      v0.1                            ║
    ║   ██║   ██║███████╗      "Because .img wasn't enough"    ║
    ║   ██║   ██║╚════██║                                      ║
    ║   ╚██████╔╝███████║                                      ║
    ║    ╚═════╝ ╚══════╝                                      ║
    ║                                                          ║
    ║   AnotherFuckingOs - Hybrid BIOS + UEFI Bootable ISO    ║
    ║                                                          ║
    ╚══════════════════════════════════════════════════════════╝
    """)


def print_usage():
    """Print usage instructions after successful build."""
    print(r"""
    ╔══════════════════════════════════════════════════════════╗
    ║  HOW TO USE YOUR SHINY NEW ISO:                          ║
    ╠══════════════════════════════════════════════════════════╣
    ║                                                          ║
    ║  QEMU (recommended for not destroying your PC):          ║
    ║                                                          ║
    ║    BIOS mode:                                            ║
    ║    $ qemu-system-x86_64 -cdrom build/AnotherFuckingOs.iso║
    ║                                                          ║
    ║    UEFI mode (needs OVMF):                               ║
    ║    $ qemu-system-x86_64 \                                ║
    ║        -bios /usr/share/OVMF/OVMF_CODE.fd \              ║
    ║        -cdrom build/AnotherFuckingOs.iso                 ║
    ║                                                          ║
    ║  VirtualBox / VMware:                                    ║
    ║    Just mount the ISO as a CD/DVD. It's not rocket       ║
    ║    science. (Well, the OS itself kind of is.)             ║
    ║                                                          ║
    ║  Real Hardware:                                          ║
    ║    Burn to CD or dd to USB. We accept no responsibility  ║
    ║    for what happens next. You've been warned.            ║
    ║                                                          ║
    ║    $ dd if=build/AnotherFuckingOs.iso of=/dev/sdX bs=4M  ║
    ║                                                          ║
    ╚══════════════════════════════════════════════════════════╝
    """)


def main():
    print_banner()
    
    # Step 0: Check tools
    check_tools()
    
    # Step 1: Build kernel & os.img
    build_kernel()
    
    # Step 2: Create BIOS boot image
    eltorito_img, emul_type = create_bios_boot_image()
    
    # Step 3: Create EFI boot image
    efi_img = create_efi_boot_image()
    
    # Step 4: Create ISO filesystem root
    create_iso_root()
    
    # Step 5: Build the ISO
    build_iso(eltorito_img, emul_type, efi_img)
    
    # Print usage
    print_usage()
    
    print(">>> Build complete. You now have a bootable ISO.")
    print(">>> Go forth and disappoint the computing world. 🔥\n")


if __name__ == "__main__":
    main()
