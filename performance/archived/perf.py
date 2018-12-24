import numpy
from mayavi.mlab import contour3d
import csv


def to_index(quality_level):
    return int(float(quality_level) * 20.0)


def load_datapoints(filename):
    datapoints = numpy.empty(shape=(21,21,21))
    with open(filename) as f:
        for row in csv.DictReader(f):
            x = to_index(row['Resolution'])
            y = to_index(row['MSAA'])
            z = to_index(row['LOD'])
            draw_time = row['Draw']
            datapoints[x, y, z] = draw_time
    return datapoints


def make_contour(datapoints):
    return contour3d(datapoints, contours=16, transparent=True)


make_contour(load_datapoints("performance/brute-sponza-fast-2018-05-02-01-06-07.csv"))
