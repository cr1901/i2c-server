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
data and use cases. Changes to protocol will _not_ be backward compatible as
per [semver](https://semver.org). The current protocol version (`v0.1`) is
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

|Code Word       |Type    |Interpretation                                                        |
|----------------|--------|----------------------------------------------------------------------|
|0               |Diff    |Zero change from previous sample.                                     |
|100             |Diff    |+1 change from previous sample.                                       |
|101             |Diff    |-1 change from previous sample.                                       |
|100 sxxxxxxxxxxx|Absolute|12-bit signed absolute sample.                                        |
|111 sxxxxxxxxxxx|Diff    |12-bit signed delta (except for values described below).              |
|111 100000000000|Event   |No value/no measurement taken this sample.<sup>1</sup>                |
|111 000000000000|Event   |Equivalent to 0. Reserved. Probably "clock went backwards".           |
|111 000000000001|Event   |Equivalent to 100. Reserved. Probably "long term jitter error".       |
|111 111111111111|Event   |Equivalent to 101. Reserved.  Probably "user event".                  |

1. A diff of the max negative value specifically should be rare enough that
   it's worth replacing its compressed form with user-event.

### Sample Rate
The sample rate is not specified in the compression- it should be sent
out-of-band. but samples should nominally be evenly spaced apart. Errors can be
used to signify skipped samples, either due to:
* Sensor read error
* High jitter on a low-precision time source, e.g. a clock with one-second precision that claims 2 seconds have elapsed.
* Leap seconds (such as a clock that has gone backward).

It is probably okay in most cases for the receiver to reconstruct
missing samples via k-nearest neighbor.
