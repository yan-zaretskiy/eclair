import datetime

import msgpack

from ecl_summary import EclSummary

# read the MessagePack binary file
with open("SPE10.mpk", "rb") as fp:
    # raw_bytes = fp.read()
    unpacker = msgpack.Unpacker(fp, use_list=False)
    print(unpacker.read_array_header())
    # for unpacked in unpacker:
    #     print(unpacked, "\n")

# unpack raw bytes


# summary_data = msgpack.unpackb(raw_bytes, use_list=False)
#
# ecl_summary = EclSummary(start_date=datetime.date(*reversed(summary_data[0])))
#
# print(ecl_summary.start_date)
