"""
Generates binary files for testing.
"""

import struct


# A single binary record of doubles

filename = 'single_record.bin'
data = [float(v) for v in range(1, 11)]
length = len(data)
fmt = f'>i{length}di'

marker = length * 8  # 8 bytes per double

with open(filename, 'wb') as f:
    f.write(struct.pack(fmt, marker, *data, marker))


# A single short (no chunks) data array of 5 8-char strings

filename = 'single_data_array.bin'
header = [b'KEYWORDS', 5, b'CHAR']
marker = 16
fmt = '>i8si4si'

with open(filename, 'wb') as f:
    f.write(struct.pack(fmt, marker, *header, marker))

data = [b'FOPR    ', b'FGPR    ', b'FWPR    ', b'WOPR    ', b'WGPR    ']
marker = 5 * 8
fmt = f">i{'8s'*5}i"
with open(filename, 'ab') as f:
    f.write(struct.pack(fmt, marker, *data, marker))
