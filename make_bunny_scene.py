import json

vertices = []
indices = []

with open("bunny.obj", "r") as file:
    for line in file:
        tokens = line.split()
        if tokens[0] == "v":
            vertices.append([-float(tokens[1]), float(tokens[2]), float(tokens[3])])
        elif tokens[0] == "f":
            indices.append([int(token) - 1 for token in tokens[1:4]])

bunny_scene = {}
bunny_scene["camera"] = {
    "film_dimensions": [1920, 1080], 
    "origin": [0, 0.7, -3],
    "look_at": [0, 0, 5.701],
    "up": [0, 1, 0],
    "field_of_view": 45
    }

bunny_scene["settings"] = {
    "environment": [0.4, 0.6, 0.9],
    "samples_per_pixel": 16,
    "max_ray_depth": 8
    }

bunny_scene["objects"] = {"floor": {
    "type": "mesh",
    "vertices": [[-10, 0, -10], [10, 0, -10], [10, 0, 10], [-10, 0, 10]],
    "indices": [[0, 1, 2], [0, 2, 3]],
    "material": {"type": "diffuse", "reflectance": [0.5, 0.3, 0.4]}
    }}

bunny_scene["objects"]["bunny"] = {
    "type": "mesh",
    "vertices": vertices,
    "indices": indices,
    "material": {"type": "diffuse", "reflectance": [0.8, 0.3, 0.1]}
    }


with open("bunny.json", "w") as output:
    json.dump(bunny_scene, output)
