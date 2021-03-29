# `compress`
`compress` is a compression algorithm optimized for I2C sensors which take
low-speed (couple of Hz) measurements over long periods of time.


## Design Notes
`compress` was designed ad hoc [over tweets](https://twitter.com/field_hamster/status/1283204695247347712)
with the help of Twitter friend [field_hamster](https://twitter.com/field_hamster).
_As of version `v0.1`, `compress` is only immediately useful for my immediate
use case,_ which was to take temperature measurements at my workbench to see
how room temperature changes over the course of a week.

As designing a protocol over tweets isn't as ideal as designing one using a [diner placemat](http://doc.cat-v.org/bell_labs/utf-8_history),
I will tweak the protocol as necessary to further optimize compression of real-world
data and use cases. Changes to protocol will _not_ be backward compatible;
an incompatible protocol receives a [semver](https://semver.org) version bump,
even if the API remains the same. The current protocol version (`v0.2`) is
documented below.

This implementation is a joint effort between [myrrlyn](https://twitter.com/myrrlyn)
([`bitvec`](https://github.com/myrrlyn/bitvec)) and I (cr1901). At present,
`compress` depends on the to-be-released version `v0.18` of `bitvec`, as I
depend on the `BitArray` type.

## Use Cases
`compress` is not inherently tied to any application or sensor. However, a user
will get the greatest space savings (upwards of 90% reduction in memory usage
after compression) on sensors whose output changes infrequently relative to the
sample rate. Temperature sensors are a good example.

Using my example above: even at the highest precision of 12-bits, the
temperature of my room changes so slowly that in the course of a full days
worth of (84,000) samples, the difference between consecutive raw values
of my temperature sensor never exceeded `+/-1`!

## Compression Algorithm
`compress` is a [prefix code](https://en.wikipedia.org/wiki/Prefix_code). Code
words can either represent raw (up-to) signed 12-bit sensor data, a difference
between the current raw data and previous data, an error, or a user event
triggered at a given sample `n`.

Each code word is stored most significant bit first, most significant byte
first. This corresponds to how compressed data is viewed in a hex editor or
via [`od`](https://en.wikipedia.org/wiki/Od_(Unix)). _Code words are packed
together without any padding bits._

### Code Words
In the below diagrams, the code words are displayed most significant bit first.
Spaces in the Code Word rows delimit prefixes.

|Code Word       |Type    |Interpretation                                                        |
|----------------|--------|----------------------------------------------------------------------|
|00              |Diff    |+1 change from previous sample.                                       |
|01              |Diff    |-1 change from previous sample.                                       |
|100 00          |Diff    |Zero change from previous sample.                                     |
|100 01          |Diff    |Zero change from previous two samples.                                |
|100 110         |Diff    |Zero change from previous five samples.                               |
|100 100         |Diff    |Zero change from previous six samples.                                |
|100 101         |Diff    |Zero change from previous seven samples.                              |
|100 111         |Diff    |Zero change from previous eight samples.                              |
|101 rrrrrrrr    |Diff    |Zero change in "r + 1" samples, run-length encoded. From 17-256.      |
|101 00000010    |Event   |No value/no measurement taken this sample.                            |
|101 00000000    |Event   |Reserved. Probably "clock went backwards".                            |
|101 00000001    |Event   |Reserved. Probably "long term jitter error".                          |
|101 00000011    |Event   |Reserved. Probably "user event".                                      |
|110 sxxxxxxxxxxx|Absolute|12-bit signed absolute sample.                                        |
|111 00          |Diff    |Zero change from previous three samples.                              |
|111 01          |Diff    |Zero change from previous four samples.                               |
|111 100         |Diff    |Zero change from previous nine samples.                               |
|111 101         |Diff    |Zero change from previous 10  samples.                                |
|111 1100        |Diff    |Zero change from previous 11 samples.                                 |
|111 1101        |Diff    |Zero change from previous 12 samples.                                 |
|111 11100       |Diff    |Zero change from previous 13 samples.                                 |
|111 11101       |Diff    |Zero change from previous 14 samples.                                 |
|111 11111       |Diff    |Zero change from previous 15 samples.                                 |
|111 11110       |Diff    |Zero change from previous 16 samples.                                 |

### Design Remarks
1. The Run-Length Encoded zero encoding was based on the taking sample data
   from my room over 24 hours, looking at the distribution of all zero runs.
   Zero runs are approximately [expontentially distributed](https://en.wikipedia.org/wiki/Exponential_distribution),
   ranging from 1 to about 256 zeroes.

   From testing, maximum compression savings comes from giving short runs of
   zeroes (up to 16) smaller encodings, while using the RLE code word for runs
   of 17 to 256; the RLE encoding of up to 16 zeroes is reserved for events.

2. The length of the uncompresed data is not encoded in the compressed data,
   and is provided out of band (file size, number of bytes read from a socket,
   etc). Reading an incompletely-filled byte of data is considered an
   acceptable "end of data" marker- it will either be an invalid code word
   or one or multiple `00` code words, meaning "no change from previous
   sample".

### Sample Rate
The sample rate is not specified in the compression- it should be sent
out-of-band. but samples should nominally be evenly spaced apart. Errors can be
used to signify skipped samples, either due to:
* Sensor read error
* High jitter on a low-precision time source, e.g. a clock with one-second precision that claims 2 seconds have elapsed.
* Leap seconds (such as a clock that has gone backward).

It is probably okay in most cases for the receiver to reconstruct
missing samples via k-nearest neighbor.
