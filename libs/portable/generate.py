#!/usr/bin/env python3

import os
import optparse
import subprocess
from hashlib import md5
import brotli
import datetime
import stat

# 4GB maximum
length_count = 4
# encoding
encoding = 'utf-8'

# output: {path: (compressed_data, file_md5)}


def normalize(path: str) -> str:
    path = path.replace('\\', '/')
    while path.startswith('./'):
        path = path[2:]
    return path.lower()


def generate_md5_table(folder: str, level, exclude: str = None) -> dict:
    res: dict = dict()
    skip = normalize(exclude) if exclude else None
    excluded = False
    # os.curdir is the literal ".", so restoring it left us inside `folder`.
    curdir = os.getcwd()
    os.chdir(folder)
    try:
        for root, directories, files in os.walk('.'):
            directories.sort()
            for entry in directories + files:
                path = os.path.join(root, entry)
                metadata = os.stat(path, follow_symlinks=False)
                if os.path.islink(path) or getattr(metadata, 'st_file_attributes', 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT:
                    raise ValueError(f"package cannot follow a reparse point: {path}")
            for filename in sorted(files):
                full_path = os.path.join(root, filename)
                if skip and normalize(full_path) == skip:
                    print(f"Excluding {full_path}...")
                    excluded = True
                    continue
                print(f"Processing {full_path}...")
                with open(full_path, "rb") as source:
                    content = source.read()
                content_compressed = brotli.compress(content, quality=level)
                md5_code = md5(content).hexdigest().encode(encoding=encoding)
                res[full_path] = (content_compressed, md5_code)
    finally:
        os.chdir(curdir)
    if skip and not excluded:
        raise ValueError(f"excluded file was not found in {folder}: {exclude}")
    return res


def write_package_metadata(md5_table: dict, output_folder: str, exe: str):
    write_blob(md5_table, os.path.join(output_folder, "data.bin"), exe)


def write_blob(md5_table: dict, output_path: str, exe: str):
    if normalize(exe) != 'merdian-desk.exe':
        raise ValueError('This attended package must launch Merdian-Desk.exe.')
    if normalize(exe) not in {normalize(path) for path in md5_table}:
        raise ValueError('The branded executable is missing from the package.')
    with open(output_path, "wb") as f:
        f.write("rustdesk".encode(encoding=encoding))
        for path in md5_table.keys():
            (compressed_data, md5_code) = md5_table[path]
            data_length = len(compressed_data)
            path = path.encode(encoding=encoding)
            # path length & path
            f.write((len(path)).to_bytes(length=length_count, byteorder='big'))
            f.write(path)
            # data length & compressed data
            f.write(data_length.to_bytes(
                length=length_count, byteorder='big'))
            f.write(compressed_data)
            # md5 code
            f.write(md5_code)
        # end
        f.write("rustdesk".encode(encoding=encoding))
        # executable
        f.write(exe.encode(encoding='utf-8'))
    print(f"Metadata has been written to {output_path}")

def write_app_metadata(output_folder: str):
    output_path = os.path.join(output_folder, "app_metadata.toml")
    with open(output_path, "w") as f:
        f.write(f"timestamp = {int(datetime.datetime.now().timestamp() * 1000)}\n")
    print(f"App metadata has been written to {output_path}")

def build_portable(output_folder: str, target: str):
    current_dir = os.getcwd()
    try:
        os.chdir(output_folder)
        cmd = ["cargo", "build", "--locked", "--release", "--package", "rustdesk-portable-packer", "--bin", "merdian-desk-portable"]
        if target:
            cmd.extend(["--target", target])
        subprocess.run(cmd, check=True)
    finally:
        os.chdir(current_dir)

# Linux: python3 generate.py -f ../rustdesk-portable-packer/test -o . -e ./test/main.py
# Windows: python3 .\generate.py -f ..\rustdesk\flutter\build\windows\runner\Debug\ -o . -e ..\rustdesk\flutter\build\windows\runner\Debug\rustdesk.exe


if __name__ == '__main__':
    parser = optparse.OptionParser()
    parser.add_option("-f", "--folder", dest="folder",
                      help="folder to compress")
    parser.add_option("-o", "--output", dest="output_folder",
                      help="the root of portable packer project, default is './'")
    parser.add_option("-e", "--executable", dest="executable",
                      help="specify startup file in --folder, default is Merdian-Desk.exe")
    parser.add_option("-t", "--target", dest="target",
                      help="the target used by cargo")
    parser.add_option("-l", "--level", dest="level", type="int",
                      help="compression level, default is 9", default=9)
    parser.add_option("--package", dest="package",
                      help="write the per-customer blob to this path instead of "
                           "data.bin, and skip the cargo build. Injected into the "
                           "template's RDPKG resource so customizing needs no rebuild")
    parser.add_option("--exclude-exe", dest="exclude_exe", action="store_true",
                      default=False,
                      help="omit the executable from the blob, for a template whose "
                           "executable ships in the package instead")
    (options, args) = parser.parse_args()
    folder = options.folder or './rustdesk'
    output_folder = os.path.abspath(options.output_folder or './')

    if options.package or options.exclude_exe:
        parser.error('External package/template overrides are disabled for this attended build.')
    if not 0 <= options.level <= 11:
        parser.error('Compression level must be between 0 and 11.')
    if not options.executable:
        options.executable = 'Merdian-Desk.exe'
    if not os.path.isabs(options.executable):
        options.executable = os.path.join(folder, options.executable)
    folder_path = os.path.abspath(folder)
    exe: str = os.path.abspath(options.executable)
    try:
        in_source_folder = os.path.commonpath([folder_path, exe]) == folder_path
    except ValueError:
        in_source_folder = False
    if not in_source_folder:
        print("The executable must locate in source folder")
        exit(-1)
    if not os.path.isfile(exe) or os.path.islink(exe):
        parser.error('The branded executable must be a real file in the input bundle.')
    exe = './' + os.path.relpath(exe, folder_path).replace('\\', '/')
    if normalize(exe) != 'merdian-desk.exe':
        parser.error('The launch executable must be Merdian-Desk.exe at the bundle root.')
    print("Executable path: " + exe)
    print("Compression level: " + str(options.level))
    md5_table = generate_md5_table(
        folder, options.level, exe if options.exclude_exe else None)
    if options.package:
        write_blob(md5_table, os.path.abspath(options.package), exe)
    else:
        write_package_metadata(md5_table, output_folder, exe)
        build_portable(output_folder, options.target)
