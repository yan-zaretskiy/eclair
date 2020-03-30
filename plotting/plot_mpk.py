"""
An example script for reading the MessagePack-formatted Eclipse summary data.
"""

import matplotlib.pyplot as plt
import msgpack


def organize_summary(vectors):
    result = {'Timing': {}, 'Wells': {}, 'Groups': {}}

    for (name, unit, vid, data) in vectors:
        if vid[0] == 'Timing':
            result['Timing'][name] = (unit, data)
        elif vid[0] == 'Well':
            well_name = vid[1]
            result['Wells'].setdefault(well_name, {})[name] = (unit, data)

    return result


# read the MessagePack binary file
with open("SPE10.mpk", "rb") as fp:
    raw_bytes = fp.read()

# unpack raw bytes
summary = msgpack.unpackb(raw_bytes)

# reshuffle data
start_date = summary[0]
results = organize_summary(summary[1])

# Sample plot
x = results['Timing']['TIME']
y = results['Wells']['P2']['WOPR']
plt.plot(x[1], y[1])
plt.title('WOPR for P2')
plt.xlabel(f'TIME [{x[0]}]')
plt.ylabel(f'WOPR [{y[0]}]')
plt.grid(True)

plt.show()
