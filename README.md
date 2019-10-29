# Squash

## A compression utility based on bzip, written for fun

To use: `./squash enc file file.sq` to compress, `./squash dec file.sq file` to decompress.

Algorithm uses a burrows-wheeler transform, followed by a move-to-front transform,
followed by a form of run-length encoding, followed by algebraic encoding.

It was pretty fun to write.
