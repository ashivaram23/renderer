import json

vertices = []
indices = []

with open("monkey.obj", "r") as file:
    for line in file:
        tokens = line.split()
        if tokens[0] == "v":
            vertices.append([-float(tokens[1]), float(tokens[2]), float(tokens[3])])
        elif tokens[0] == "f":
            indices.append([int(token) - 1 for token in tokens[1:4]])

with open("monkeyverts.json", "w") as output:
    json.dump({"vertices": vertices, "indices": indices}, output)
