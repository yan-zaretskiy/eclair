# Eclipse Binary Format

## Data Arrays

Eclipse binary output files are typically written using the **big-endian** ordering. A single binary block is written in a Fortran style, where actual binary data is surrounded by equal leading and tailing record markers. The marker is a 4-byte integer (`int32`), whose value is the length of the data in bytes. For instance, if we have a binary block of 200 bytes, it will be written to disk as:

```
+-------+----------+-------+
|  200  |   data   |  200  |
+-------+----------+-------+
```

A single *data array* consists of a header and a body section, written as two individual binary blocks.

The header section contains the following items:

1. An 8-character keyword for what the data corresponds to;
2. A 4-byte integer for the number of elements in the block;
3. A 4-character keyword defining the type of data;

Possible data type values are:

- `INTE` - 4-byte signed integers;
- `REAL` - single precision 4-byte floating point numbers;
- `DOUB` - double precision 8-byte floating point numbers;
- `LOGI` - 4-byte logicals;
- `CHAR` - characters (as 8-character words);
- `C0nn` - CHARACTER*nn strings (e.g. C042 means a 42-character string);
- `MESS` - an indicator type, it contains no data, so its length is zero;

As for the body section, it is written in batches of one or more sub-blocks of either 1000 non-string items or 105 8-character words.

Here is how a data array is laid out on disk if it is 1500 integers long and it is called `FOO`:

```
+------+---------------------+------+------+-----------------+------+------+--------------------+------+
| head | KEYWORD LENGTH TYPE | tail | head | VAL1 .. VAL1000 | tail | head | VAL1001 .. VAL1500 | tail |
+------+---------------------+------+------+-----------------+------+------+--------------------+------+
|  16  | FOO     1500   INTE |  16  | 4000 |    1   ..  1000 | 4000 | 2000 |    1001 ..    1500 | 2000 |
+------+---------------------+------+------+-----------------+------+------+--------------------+------+
```

Note that `FOO` will be padded with spaces to be exactly 8 characters long.
