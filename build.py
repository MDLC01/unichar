#!/usr/bin/env python3


import os
import shutil
import subprocess
import sys
import urllib.request

from pathlib import Path


LIBRARY_DIR = Path('src/')
TARGET_DIR = Path('target/')
GENERATED_DIR = TARGET_DIR.joinpath('ucd/')
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


def build_ucd():
    print('Building Unicode Character Database...')

    blocks_url = 'https://www.unicode.org/Public/UCD/latest/ucd/Blocks.txt'
    unicode_data_url = 'https://www.unicode.org/Public/UCD/latest/ucd/UnicodeData.txt'
    # Applicable as per https://www.unicode.org/copyright.html.
    license_url = 'https://www.unicode.org/license.txt'

    # Get block list.
    # https://unicode.org/reports/tr44/#Blocks.txt
    blocks = []
    for block_range, name in read_unicode_data_file(blocks_url):
        [first, last] = block_range.split('..')
        blocks.append((first, int(first, base=16), int(last, base=16), name))

    # Get data for the codepoints of each block.
    block_contents = [[None] * (last - first + 1) for _, first, last, _ in blocks]
    # https://unicode.org/reports/tr44/#UnicodeData.txt
    for (
        cp,
        name,
        general_category,
        canonical_combining_class,
        bidi_class,
        # https://unicode.org/reports/tr44/#Decomposition_Type
        decomposition,
        # https://unicode.org/reports/tr44/#Numeric_Type
        _,
        _,
        numeric_value,
        bidi_mirrored,
        _unicode_1_name,
        _iso_comment,
        simple_uppercase_mapping,
        simple_lowercase_mapping,
        # Note that if empty, this should fall back to Simple_Uppercase_Mapping.
        simple_titlecase_mapping,
    ) in read_unicode_data_file(unicode_data_url):
        cp = int(cp, base=16)
        for block_content, (_, block_first, block_last, _) in zip(block_contents, blocks):
            if block_first <= cp <= block_last:
                block_content[cp - block_first] = f'({', '.join([
                    f'"{name}"',
                    f'"{general_category}"',
                    f'{canonical_combining_class}',
                ])})'
                break
        else:
            raise ValueError('Valid codepoint outside block.')

    # Clean-up data.
    for block_content in block_contents:
        while block_content[-1] is None:
            block_content.pop()

    GENERATED_DIR.mkdir(parents=True)

    with open(GENERATED_DIR.joinpath('LICENSE'), 'wb') as f:
        f.writelines(urllib.request.urlopen(license_url))

    is_block_sparse = []
    for (block_id, _, _, name), content in zip(blocks, block_contents):
        with open(GENERATED_DIR.joinpath(f'block-{block_id}.typ'), 'w') as f:
            f.write('#let data = (\n')
            # If the block is sparse, we use a dictionary instead of an array.
            if content.count(None) > len(content) / 2:
                is_block_sparse.append(True)
                for i, entry in enumerate(content):
                    if entry is not None:
                        f.write(f'  "{i:x}": {entry},\n')
            else:
                is_block_sparse.append(False)
                for entry in content:
                    if entry is None:
                        entry = '()'
                    f.write(f'  {entry},\n')
            f.write(')\n')

    with open(GENERATED_DIR.joinpath('index.typ'), 'w') as f:
        f.write('#let get-data(code) = {\n  ')
        for (block_id, first, last, name), is_sparse in zip(blocks, is_block_sparse):
            if is_sparse:
                key = f'str(code - 0x{first:x}, base: 16)'
            else:
                key = f'code - 0x{first:x}'
            f.write(f'if 0x{first:x} <= code and code <= 0x{last:x} {{\n')
            f.write(f'    import "block-{block_id}.typ"\n')
            f.write(f'    ("{name}", block-{block_id}.data.at({key}, default: ()))\n')
            f.write('  } else ')
        f.write('{\n    (none, ())\n  }\n}\n')


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
    example = []
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
            final_lines.append(f'![image]({example_path(example_count).as_posix()})')
        elif is_example:
            if line.startswith('%'):
                example.append('#' + line[1:])
            else:
                final_lines.append(line)
                example.append(line)
        elif line.startswith('```example'):
            final_lines.append('```typ')
            is_example = True
            example = []
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
    build_ucd()
    build_readme()


if __name__ == '__main__':
    try:
        main()
    except subprocess.CalledProcessError:
        exit(1)
