#!/usr/bin/env python3

import argparse
import os
import os.path as p
import shutil
import subprocess
import tempfile


def resize(filename, args):
    st = os.stat(filename)
    mode = st.st_mode & 0o666
    orig_size = st.st_size
    base = p.basename(filename)
    (main, _ext) = p.splitext(base)
    outfile = p.join(args.output_dir, main + '.jpg')
    t1 = tempfile.NamedTemporaryFile(prefix=main, suffix='.jpg')
    subprocess.check_call([
        'convert', '-resize', '>{0}x{0}'.format(args.max_size), '-quality',
        str(args.quality), filename, t1.name])
    t2 = tempfile.NamedTemporaryFile(prefix=main, suffix='.jpg', delete=False)
    subprocess.check_call(
        ['jpegtran', '-optimize', '-progressive', t1.name],
        stdout=t2)
    if os.stat(t2.name).st_size < orig_size * 0.95:
        shutil.move(t2.name, outfile)
        os.chmod(outfile, mode)
        print(outfile)
    else:
        os.unlink(t2.name)
        print('{} (skipped) '.format(outfile))


def main():
    a = argparse.ArgumentParser()
    a.add_argument(
        '-o', '--output-dir', default='resized',
        help='write resized images to DIR [default: "resized"]')
    a.add_argument(
        '-m', '--max-size', metavar='SIZE', type=int, default=3840,
        help='rescale images so that the longes dimension is no more than '
        'SIZE pixels [default: 3840]')
    a.add_argument(
        '-q', '--quality', metavar='%', type=int, default=80,
        help='JPEG compression quality (1..100) [default: 80]')
    a.add_argument('FILE', nargs='+', help='raw image files (JPEG)')

    args = a.parse_args()
    os.makedirs(args.output_dir, exist_ok=True)
    for f in args.FILE:
        resize(f, args)


if __name__ == '__main__':
    main()
