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
    temps_bytestr = base64.b64decode(json_data["buf"])
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
