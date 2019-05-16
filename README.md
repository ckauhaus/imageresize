imageresize
===========

Handy tool for reducing masses of JPEG files to resonable file sizes. Scales
images on all available CPUs in parallel. Preserves EXIF metadata.

Usage example:
```sh
imageresize *.jpg
```

Scaled images end up in a `resized` subdirectory. Use the `-o` option to specify
another destination.

Author: [Christian Kauhaus](mailto:christian@kauhaus.de)

Licensed under the terms of the
[BSD 3-Clause "Revised" License](https://opensource.org/licenses/BSD-3-Clause).
