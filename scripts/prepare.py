#!/usr/bin/env python3
"""Fetch a pinned Servo and apply a small, reviewable shell patch."""
import argparse
import json
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOCK = json.loads((ROOT / 'servo.lock.json').read_text())

def run(*args, cwd=None):
    subprocess.run(args, cwd=cwd, check=True)

def replace_once(root, name, old, new):
    path = root / name
    text = path.read_text(encoding='utf-8')
    if text.count(old) != 1:
        raise RuntimeError(f'Pinned source mismatch in {name}: expected exactly one patch anchor')
    path.write_text(text.replace(old, new), encoding='utf-8', newline='\n')

def patch(root):
    shell = 'ports/servoshell/'
    replace_once(root, shell + 'lib.rs', 'mod parser;', 'mod parser;\n#[cfg(not(any(target_os = "android", target_env = "ohos")))]\nmod ytm;\n#[cfg(not(any(target_os = "android", target_env = "ohos")))]\nmod ytm_media;')
    replace_once(root, shell + 'desktop/cli.rs',
                 'let args: Vec<String> = env::args().skip(1).collect();',
                 'let args = crate::ytm::arguments(env::args().skip(1).collect());')
    replace_once(root, shell + 'desktop/app.rs', 'let servo = servo_builder.build();',
                 'let servo = servo_builder.build();\n        crate::ytm::initialize();')
    replace_once(root, shell + 'prefs.rs', 'homepage: "https://servo.org".into(),',
                 'homepage: "https://music.youtube.com/".into(),')
    replace_once(root, shell + 'prefs.rs', 'fallback(String::from("https://www.servo.org"))',
                 'fallback(String::from("https://music.youtube.com/"))')
    replace_once(root, shell + 'prefs.rs', 'config_dir.push("servo");', 'config_dir.push("ytm-servo");')
    # There are separate macOS and Windows branches. Both intentionally use a dedicated profile.
    path = root / shell / 'prefs.rs'
    text = path.read_text().replace('config_dir.push("Servo");', 'config_dir.push("ytm-servo");\n    config_dir.push("default");')
    path.write_text(text, encoding='utf-8', newline='\n')
    replace_once(root, shell + 'prefs.rs', 'let default_window_size = Size2D::new(1024, 740);',
                 'let default_window_size = Size2D::new(1280, 820);')
    anchor = '                let frame = egui::Frame::default()'
    replacement = '''                Panel::top("ytm_app").show_inside(ctx, |ui| {
                    if crate::ytm::toolbar(ui) {
                        window.queue_user_interface_command(UserInterfaceCommand::Go(crate::ytm::HOME.into()));
                    }
                });
                let frame = egui::Frame::default()'''
    replace_once(root, shell + 'desktop/gui.rs', anchor, replacement)
    replace_once(root, shell + 'Cargo.toml', '[dependencies]\n', '[dependencies]\ngstreamer = { workspace = true }\n')
    replace_once(root, shell + 'Cargo.toml', 'FileDescription = "ServoShell"', 'FileDescription = "YTM Servo (experimental)"')
    replace_once(root, shell + 'Cargo.toml', 'ProductName = "ServoShell"', 'ProductName = "YTM Servo"')
    # gstreamer already exists in the pinned lock; add only this dependency edge.
    lock = root / 'Cargo.lock'
    text = lock.read_text()
    begin = text.index('name = "servoshell"\n')
    end = text.find('\n[[package]]', begin)
    block = text[begin:end]
    if ' "gstreamer",' in block: raise RuntimeError('Unexpected pre-existing direct dependency')
    block = block.replace(' "glow",\n', ' "glow",\n "gstreamer",\n')
    if ' "gstreamer",' not in block: raise RuntimeError('Could not patch dependency lock')
    lock.write_text(text[:begin] + block + text[end:], encoding='utf-8', newline='\n')
    shutil.copyfile(ROOT / 'shell/ytm.rs', root / shell / 'ytm.rs')
    shutil.copyfile(ROOT / 'src/lib.rs', root / shell / 'ytm_media.rs')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--servo', type=Path, default=ROOT / 'vendor/servo')
    parser.add_argument('--existing', action='store_true', help='Use an existing clean checkout at the pinned revision')
    args = parser.parse_args()
    root = args.servo.resolve()
    if not args.existing:
        if root.exists(): raise SystemExit(f'{root} exists; use --existing on a clean pinned checkout')
        root.mkdir(parents=True)
        run('git', 'init', str(root))
        run('git', 'remote', 'add', 'origin', LOCK['repository'], cwd=root)
        run('git', 'fetch', '--depth', '1', 'origin', LOCK['revision'], cwd=root)
        run('git', 'checkout', '--detach', 'FETCH_HEAD', cwd=root)
    actual = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    if actual != LOCK['revision']: raise SystemExit(f'Expected {LOCK["revision"]}, found {actual}')
    status = subprocess.check_output(['git', 'status', '--porcelain', '--untracked-files=normal'], cwd=root, text=True)
    if status.strip(): raise SystemExit('Refusing to modify a dirty Servo checkout')
    patch(root)
    print(f'Prepared YTM Servo at {root}; engine {actual}')

if __name__ == '__main__': main()
