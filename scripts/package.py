#!/usr/bin/env python3
"""Package a native Servo build; Linux uses system GStreamer, Windows bundles it."""
import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import tarfile
import zipfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]

def copy_tree(source, destination):
    if not source.is_dir(): raise RuntimeError(f'Missing required directory: {source}')
    shutil.copytree(source, destination, dirs_exist_ok=True)

def windows_gstreamer(servo):
    # Same resolution order as pinned python/servo/platform/windows.py.
    candidates = [servo / 'target/dependencies/gstreamer/1.0/msvc_x86_64',
                  servo / 'target/dependencies/gstreamer/1.0/msvc_X86_64']
    if os.environ.get('GSTREAMER_1_0_ROOT_MSVC_X86_64'):
        candidates.append(Path(os.environ['GSTREAMER_1_0_ROOT_MSVC_X86_64']))
    candidates.append(Path('C:/gstreamer/1.0/msvc_x86_64'))
    for candidate in candidates:
        if (candidate / 'bin/gstreamer-1.0-0.dll').is_file(): return candidate
    raise RuntimeError('Could not find bootstrapped GStreamer runtime')

def package(servo, target, output):
    arch = target.split('-')[-1]
    windows = target.startswith('windows-')
    build = servo / 'target/release'
    stage = output / f'ytm-servo-{target}'
    if stage.exists(): raise RuntimeError(f'Refusing to overwrite stage: {stage}')
    stage.mkdir(parents=True)
    binary_name = 'servoshell.exe' if windows else 'servoshell'
    app_name = 'ytm-servo.exe' if windows else 'ytm-servo'
    shutil.copy2(build / binary_name, stage / app_name)
    copy_tree(servo / 'resources', stage / 'resources')
    copy_tree(servo / 'components/default-resources/resources', stage / 'resources/named_resources')
    for name in ['README.md', 'LICENSE', 'THIRD_PARTY.md', 'servo.lock.json']:
        shutil.copy2(ROOT / name, stage / name)
    copy_tree(ROOT / 'docs', stage / 'docs')
    copy_tree(ROOT / 'packaging', stage / 'packaging')
    (stage / 'licenses').mkdir()
    for name in ['LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE', 'COPYRIGHT']:
        if (servo / name).is_file(): shutil.copy2(servo / name, stage / 'licenses' / ('servo-' + name))
    # Record source provenance separately from runtime-generated diagnostics.
    (stage / 'BUILD.json').write_text(json.dumps({
        'engine': json.loads((ROOT / 'servo.lock.json').read_text()),
        'application_commit': os.environ.get('GITHUB_SHA', 'local'),
        'target': target, 'playback_verified': False,
    }, indent=2) + '\n')
    if windows:
        for dll in build.glob('*.dll'): shutil.copy2(dll, stage / dll.name)
        gst = windows_gstreamer(servo)
        # Full plugin runtime: required for hardware codecs excluded by Servo's minimal bundle.
        for dll in (gst / 'bin').glob('*.dll'): shutil.copy2(dll, stage / dll.name)
        for dll in (gst / 'lib/gstreamer-1.0').glob('*.dll'): shutil.copy2(dll, stage / dll.name)
        for program in ['gst-inspect-1.0.exe', 'gst-launch-1.0.exe']:
            shutil.copy2(gst / 'bin' / program, stage / program)
        for scanner in gst.glob('libexec/gstreamer-1.0/gst-plugin-scanner.exe'):
            shutil.copy2(scanner, stage / scanner.name)
        for directory in ['licenses', 'share/licenses', 'share/doc']:
            source = gst / directory
            if source.is_dir(): copy_tree(source, stage / 'licenses/gstreamer' / directory)
        archive = output / f'{stage.name}.zip'
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as z:
            for path in sorted(stage.rglob('*')):
                if path.is_file(): z.write(path, Path(stage.name) / path.relative_to(stage))
    else:
        (stage / app_name).chmod(0o755)
        # Fail packaging on missing ELF dependencies in the build environment.
        linked = subprocess.run(['ldd', str(stage / app_name)], capture_output=True, text=True, check=True).stdout
        if 'not found' in linked: raise RuntimeError(linked)
        (stage / 'ELF-DEPENDENCIES.txt').write_text(linked)
        archive = output / f'{stage.name}.tar.gz'
        with tarfile.open(archive, 'w:gz') as tar: tar.add(stage, arcname=stage.name)
        deb_arch = {'x86_64': 'amd64', 'arm64': 'arm64'}[arch]
        deb = output / f'deb-{deb_arch}'
        copy_tree(stage, deb / 'opt/ytm-servo')
        (deb / 'usr/bin').mkdir(parents=True)
        launcher = deb / 'usr/bin/ytm-servo'
        launcher.write_text('#!/bin/sh\nexec /opt/ytm-servo/ytm-servo "$@"\n')
        launcher.chmod(0o755)
        (deb / 'usr/share/applications').mkdir(parents=True)
        shutil.copy2(ROOT / 'packaging/ytm-servo.desktop', deb / 'usr/share/applications')
        (deb / 'usr/share/icons/hicolor/scalable/apps').mkdir(parents=True)
        shutil.copy2(ROOT / 'packaging/ytm-servo.svg', deb / 'usr/share/icons/hicolor/scalable/apps')
        (deb / 'DEBIAN').mkdir()
        control = (ROOT / 'packaging/control.in').read_text().replace('@ARCH@', deb_arch)
        (deb / 'DEBIAN/control').write_text(control)
        subprocess.run(['dpkg-deb', '--root-owner-group', '--build', str(deb), str(output / f'{stage.name}.deb')], check=True)
    return stage

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--servo', type=Path, default=ROOT / 'vendor/servo')
    parser.add_argument('--target', required=True, choices=['windows-x86_64', 'linux-x86_64', 'linux-arm64'])
    parser.add_argument('--output', type=Path, default=ROOT / 'dist')
    args = parser.parse_args()
    expected = 'arm64' if platform.machine().lower() in ['aarch64', 'arm64'] else 'x86_64'
    if not args.target.endswith(expected): raise SystemExit('Package target does not match native build host')
    stage = package(args.servo.resolve(), args.target, args.output.resolve())
    print(stage)
    artifacts = [p for p in args.output.iterdir() if p.is_file() and (p.name.endswith(('.zip', '.tar.gz', '.deb')))]
    checksums = ''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n' for p in sorted(artifacts))
    (args.output / f'SHA256SUMS-{args.target}.txt').write_text(checksums)
if __name__ == '__main__': main()
