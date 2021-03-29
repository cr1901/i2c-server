#%matplotlib inline

import json
import base64
import struct
import argparse

from matplotlib import pyplot as plt
import numpy as np
import scipy.signal as sp
import requests

#%%
def grab_data_url(url, out_fn):
    req = requests.get(url)
    json_data = json.loads(req.text)
    with open(out_fn, "w") as fp:
        json.dump(json_data, fp)
    return json_data

def grab_data_file(in_fn):
    with open(in_fn) as fp:
        json_data = json.load(fp)
    return json_data

#%%
def process_data(json_data, use_timestamp=False, windows=[15, 30, 60]):
    temps_bytestr = base64.b64decode(json_data["buf"], altchars="-_")
    temp = [(1.8*(t/16.0) + 32) for t in struct.unpack_from("<"+"h"*(len(temps_bytestr)//2), temps_bytestr)]
    time = [i for i in range(len(temp))]

    reltime = time # TODO: Actually calculate timestamps

    avg_temps = [temp]

    if windows != [0]:
        for w in windows:
            avg_temps.append({"width" : w, "data" : np.convolve(temp, np.ones((w,))/w, mode='valid')})

    return (reltime, avg_temps)

#%%
def write_plot(time, avg_temps, use_timestamp=False, marker_size=1):
    prop_cycle = plt.rcParams['axes.prop_cycle']
    colors = prop_cycle.by_key()['color']

    fig, ax = plt.subplots(figsize=(10, 8))

    if use_timestamp:
        xlabel = "Timestamp"
    else:
        xlabel = "Relative Time (s)"

    ax.set(xlabel=xlabel,
           ylabel="Temp (F)",
           title="Average Workbench Temperature")
    ax.grid(color=colors[0], linestyle=":")
    ax.yaxis.label.set_color(colors[0])
    ax.tick_params(axis="y", colors=colors[0])

    ax2 = ax.twinx()
    ax2.set(ylabel="Temp (C)")
    ax2.grid(color=colors[1], linestyle=":")
    ax2.yaxis.label.set_color(colors[1])
    ax2.tick_params(axis="y", colors=colors[1])

    raw, = ax.plot(time, avg_temps[0], '.', color=colors[3], markersize=marker_size)

    lines = [raw]
    legend_labels = ["raw data"]
    for d, color in zip(avg_temps[1:], [colors[2]] + colors[4:]):
        lines.append(ax.plot(time[d["width"]-1:], d["data"], "-", color=color, linewidth=1)[0])
        legend_labels.append("{} smp avg".format(d["width"]))

    ax.legend(lines, legend_labels)

    def to_celsius(tf):
        return (tf-32)/1.8

    y1, y2 = ax.get_ylim()
    ax2.set_ylim(to_celsius(y1), to_celsius(y2))
    return fig

#%%
# https://stackoverflow.com/questions/24885092/finding-the-consecutive-zeros-in-a-numpy-array
def zero_runs(a):
    # Create an array that is 1 where a is 0, and pad each end with an extra 0.
    iszero = np.concatenate(([0], np.equal(a, 0).view(np.int8), [0]))
    absdiff = np.abs(np.diff(iszero))
    # Runs start and end where absdiff is 1.
    ranges = np.where(absdiff == 1)[0].reshape(-1, 2)
    return ranges

#%%
def write_hist(zeros):
    fig, ax = plt.subplots(figsize=(10, 8))

    ax.set(xlabel="Number of zeros",
           ylabel="Total bit count",
           title="Zero runs historgram")
    ax.hist(zeros, bins=max())

    return fig

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Plot I2C Data from I2C Server.")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("-u", "--url", type=str, help="URL to retrieve JSON data (make sure to include http://)")
    group.add_argument("-j", "--json-in", type=str, help="Read JSON from file.")
    parser.add_argument("-d", "--json-out", type=str, default="dump.json", help="Dump JSON to file from URL (does nothing when --json-in is supplied). Default: dump.json.")
    parser.add_argument("-w", "--windows", type=int, nargs="+", metavar="N", default=[15, 30, 60], help="Plot sliding window averages over N seconds. '0' means 'do not plot averages'. Default: [15, 30, 60].")
    parser.add_argument("-t", "--timestamp", action="store_true", help="Use timestamps in plot instead of starting at 0 seconds.")
    parser.add_argument("-m", "--marker-size", type=int, default=1, help="Marker size of the raw data. Default: 1.")
    parser.add_argument("-f", "--figure-out", type=str, default="plot.png", help="Output plot filename. Default: plot.png.")
    parser.add_argument("-z", "--histogram", type=str, default="zero.png", help="Histogram of runs of constant data. Default: zero.png.")
    args = parser.parse_args()

    print("Grabbing data...")
    if args.url:
        json_data = grab_data_url(args.url, args.json_out)
    else:
        json_data = grab_data_file(args.json_in)

    print("Processing data...")
    time, avg_temps = process_data(json_data, use_timestamp=args.timestamp, windows=args.windows)

    print("Creating plot...")
    fig = write_plot(time, avg_temps, use_timestamp=args.timestamp, marker_size=args.marker_size)
    fig.savefig(args.figure_out)

    print("Generating distribution...")
    diffs = np.diff(avg_temps[0])
    runs = zero_runs(diffs)
    run_length = runs[:,1] - runs[:,0]
    rl_values, rl_counts = np.unique(run_length, return_counts=True)

    print("  {} total measurements".format(len(avg_temps[0])))
    print("  {} unique zero runs".format(np.sum(rl_counts)))
    print("  {} total zero bits".format(np.sum(rl_values * rl_counts)))
    print("  {} total non-zero diffs".format(len(diffs[diffs != 0.0])))
    rl_counts_norm = rl_counts / np.sum(rl_counts)

    hfig, ax = plt.subplots(figsize=(10, 8))
    ax.set(xlabel="Number of zeros in run",
           ylabel="Proportion of total",
           title="Zero runs bit count")
    ax.bar(rl_values , rl_counts_norm)

    hfig.savefig(args.histogram)

    print("Entropy calculations...")
    theoretical_entropy = np.sum(rl_counts_norm*-np.log2(rl_counts_norm))
    print("  Theoretical: {} bits/symbol".format(theoretical_entropy))

    # The length of the run == the number of zero bits stored per run.
    # This is only mildly more efficient than 8-bit RLE for all values, even
    # _with_ the extra "0" prefix bit (avg. 9 bits/symbol)!
    bits_per_codeword_no_rle = np.sum(rl_values * rl_counts_norm)
    print("  No RLE: {} bits/symbol".format(bits_per_codeword_no_rle))

    # 00, 010, 0110
    # 0111: run-length.
    rle_bits_required = 3 + 8
               # 1  2  3  4  5  6  7  8  9  10 11 12 13  14  15  16
    rl_start =  [4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 10, 10]
    rl_rest = [rle_bits_required] * len(rl_values[(rl_values > len(rl_start))])

    bits_per_codeword_rle = np.sum(np.concatenate((rl_start, rl_rest)) * rl_counts_norm)
    print("  RLE: {} bits/symbol".format(bits_per_codeword_rle))

    print("Rough compression estimate...")
    print("Assumes: RLE, one single leading absolute measurement")
    initial_abs_bits = 15
    bits_per_measurement = 12
    nonzero_bits = 2

    total_zero_bits = np.sum(np.concatenate((rl_start, rl_rest)) * rl_counts)
    # 4 bits per +/-1 increment.
    total_nonzero_bits = np.sum(nonzero_bits*len(diffs[diffs != 0.0]))
    total_compressed_bits = total_zero_bits + total_nonzero_bits + initial_abs_bits
    uncompressed_bits = bits_per_measurement*len(avg_temps[0])

    print("  Zero bits: {}".format(total_zero_bits))
    print("  Nonzero bits: {}".format(total_nonzero_bits))
    print("  Compression ratio: {}".format(total_compressed_bits/uncompressed_bits))
