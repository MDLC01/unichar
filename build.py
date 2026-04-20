#!/usr/bin/env python3


import os
import shutil
import subprocess
import sys
import urllib.request

from pathlib import Path


LIBRARY_DIR = Path('src/')
PLUGIN_DIR = Path('plugin/')
TARGET_DIR = Path('target/')
LICENSE = Path('LICENSE')
CHANGELOG = Path('CHANGELOG.md')
README = 'README.md'


def delete_directory_content(directory):
    if directory.exists():
        shutil.rmtree(directory)
    directory.mkdir(parents=True)


def read_exclude_file(path: Path) -> list[str]:
    with open(path) as f:
        lines = [line.strip() for line in f.read().splitlines()]
    return [os.path.normcase(entry) for entry in lines if entry and not entry.startswith('#')]


def copy_library():
    print('Copying library...')

    README_PATH = os.path.normcase(LIBRARY_DIR.joinpath(README))

    for path, directories, files in os.walk(LIBRARY_DIR):
        for file in files:
            file_name = Path(path).relative_to(LIBRARY_DIR).joinpath(file)
            if os.path.normcase(file_name) != README_PATH:
                source = LIBRARY_DIR.joinpath(file_name)
                destination = TARGET_DIR.joinpath(file_name)
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, destination)
    shutil.copyfile(LICENSE, TARGET_DIR.joinpath('LICENSE'))


def read_unicode_data_file(url):
    entries = []
    for line in urllib.request.urlopen(url):
        line = line.decode('UTF-8').strip()
        if line and not line.startswith('#'):
            entries.append(tuple(part.strip() for part in line.split(';')))
    return entries


def codepoint_range(r):
    if '..' in r:
        [first, last] = r.split('..')
        for cp in range(int(first, base=16), int(last, base=16) + 1):
            yield cp
    else:
        yield int(r, base=16)


def build_plugin():
    print('Building plugin...')

    urllib.request.urlretrieve(
        'https://www.unicode.org/license.txt',
        TARGET_DIR.joinpath('UNICODE.LICENSE'),
    )

    BUILD_TARGET = 'wasm32-unknown-unknown'
    PLUGIN_PATH = PLUGIN_DIR.joinpath('target', BUILD_TARGET, 'release', 'unichar_plugin.wasm')
    PLUGIN_NAME = 'plugin.wasm'

    subprocess.run(
        ['cargo', 'build', '--release', '--target', BUILD_TARGET, '--quiet'],
        cwd=PLUGIN_DIR,
        check=True,
    )

    shutil.copyfile(PLUGIN_PATH, TARGET_DIR.joinpath(PLUGIN_NAME))


def build_readme():
    print('Building README...')

    EXAMPLES_DIR = Path('examples/')

    def example_path(n):
        return EXAMPLES_DIR.joinpath(f'example-{n}.svg')

    final_lines = []

    with open(LIBRARY_DIR.joinpath(README), encoding='UTF-8') as f:
        initial_readme = f.read()

    # Build examples

    example_source = [
        '#import "lib.typ": *',
        '#set page(width: auto, height: auto, margin: 0.5cm, fill: none)',
    ]
    example_count = 0
    example = None
    example_alt = None
    is_example = False
    is_start = True
    for line in initial_readme.splitlines():
        # Skip initial note.
        if is_start:
            if line.startswith('>') or line.strip() == '':
                continue
            is_start = False
        # Handle examples.
        if is_example and line.startswith('```'):
            is_example = False
            example_count += 1
            example_source.append(f'#page[\n{'\n'.join(example)}\n]')
            final_lines.append(line)
            final_lines.append('')
            final_lines.append(f'![Result of the code example: {example_alt}]({example_path(example_count).as_posix()})')
        elif is_example:
            if line.startswith('%'):
                example.append('#' + line[1:])
            else:
                final_lines.append(line)
                example.append(line)
        elif line.startswith('```example:'):
            final_lines.append('```typ')
            is_example = True
            example = []
            example_alt = line[len('```example:'):].strip()
        else:
            final_lines.append(line)

    TARGET_DIR.joinpath(EXAMPLES_DIR).mkdir(parents=True)
    subprocess.run(
        ['typst', 'compile', '-', str(example_path('{n}'))],
        input='\n'.join(example_source),
        encoding='UTF-8',
        cwd=TARGET_DIR,
        check=True,
    )

    # Add changelog

    with open(CHANGELOG, encoding='UTF-8') as f:
        initial_changelog = f.read()

    final_lines.append('')
    final_lines.append('')
    for line in initial_changelog.splitlines():
        if line.startswith('#'):
            final_lines.append('#' + line)
        else:
            final_lines.append(line)

    # Write README

    with open(TARGET_DIR.joinpath(README), 'w', encoding='UTF-8') as f:
        f.write('\n'.join(final_lines))


def main():
    delete_directory_content(TARGET_DIR)
    copy_library()
    build_plugin()
    build_readme()


if __name__ == '__main__':
    try:
        main()
    except subprocess.CalledProcessError:
        exit(1)
