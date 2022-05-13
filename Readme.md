# cbuffer #

*A terminal utilty for recording raw audio with a circular prerecord buffer.*

**Usage**  

`cbuffer [-b [BITRATE]] [SECONDS]`  

        -b, --bitrate <BITRATE> [default: 1411200]
        -h, --help              Print help information
        -V, --version           Print version information

**Dependencies**  

    rust/cargo 1.60.0  
    pulseaudio (optional)

## About

Buffers stdin for specified number of seconds and starts writing to stdout when
a key is pressed. **q** or **ctrl-c** will stop the program, any other key will
toggle between recording and buffering. Specify the bitrate so the program can
determine how much data constitutes a full second (default is for 16-bit 44.1k
2ch audio).

You could theoretically use this program to record other types of lossless,
headerless data in the same manner.

## crecord

`crecord [-hf] [-b buffer_seconds] [-s pulse_sink] filename`

Included is a bash script `crecord` which will read from a specified pulseaudio
sink using SoX and write to a file. -b specifies buffer size in seconds, -s
specifies pulse source, -f forces overwrite of an existing file.

## Installation

    make
    make install

This will install both cbuffer and the crecord script into $PREFIX/bin.

## Examples

> bit rate = **sample rate * bit depth * channels**

Record raw samples using arecord and redirect to file:  

    arecord -traw -c2 -fS16_LE -r44100 - | cbuffer -b 1411200 > test.raw

Play back using aplay with same arguments:  

    aplay -traw -c2 -fS16_LE -r44100 test.raw

Or use something like SoX to insert a WAV header:

    sox -q -t pulseaudio default.monitor -t raw -L -esigned-integer -r44100 -b16 -c2 - | \
        cbuffer -b 1411200 5 | \
        sox -t raw -L -esigned-integer -r44100 -b16 -c2 - test.wav
