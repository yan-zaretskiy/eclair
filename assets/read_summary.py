import matplotlib.pyplot as plt
import msgpack


def organize_summary(vectors):
    result = {}
    for (name, unit, vid, data) in vectors:
        if vid[0] == 'Timing':
            result.setdefault('Timing', {})[name] = (unit, data)
        elif vid[0] == 'Well':
            well_name = vid[1]
            result.setdefault('Wells', {}).setdefault(well_name, {})[name] = (unit, data)

    return result


# read the MessagePack binary file
with open("SPE10.mpk", "rb") as fp:
    raw_bytes = fp.read()

# unpack raw bytes
summary = msgpack.unpackb(raw_bytes, strict_map_key=False)

start_date = summary[0]
results = organize_summary(summary[1])

# Sample plot
plt.plot(results['Timing']['TIME'][1], results['Wells']['P2']['WOPR'][1])
plt.show()
