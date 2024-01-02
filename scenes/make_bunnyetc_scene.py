import json

bunny_scene = {}
bunny_scene["camera"] = {
    "film_dimensions": [1920, 1080], 
    "origin": [0, 0.7, -3],
    "look_at": [0, 0, 2.701],
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

bunny_vertices = []
bunny_indices = []

with open("bunnyetc-bunny.obj", "r") as file:
    for line in file:
        tokens = line.split()
        if tokens[0] == "v":
            bunny_vertices.append([-float(tokens[1]), float(tokens[2]), float(tokens[3])])
        elif tokens[0] == "f":
            bunny_indices.append([int(token) - 1 for token in tokens[1:4]])

bunny_scene["objects"]["bunny"] = {
    "type": "mesh",
    "vertices": bunny_vertices,
    "indices": bunny_indices,
    "material": {"type": "diffuse", "reflectance": [0.8, 0.3, 0.1]}
    }

monkey_vertices = []
monkey_indices = []

with open("bunnyetc-monkey.obj", "r") as file:
    for line in file:
        tokens = line.split()
        if tokens[0] == "v":
            monkey_vertices.append([-float(tokens[1]), float(tokens[2]), float(tokens[3])])
        elif tokens[0] == "f":
            monkey_indices.append([int(token) - 1 for token in tokens[1:4]])

bunny_scene["objects"]["monkey"] = {
    "type": "mesh",
    "vertices": monkey_vertices,
    "indices": monkey_indices,
    "material": {"type": "diffuse", "reflectance": [0.8, 0.8, 0.8]}
    }

teapot_vertices = []
teapot_indices = []

with open("bunnyetc-teapot.obj", "r") as file:
    for line in file:
        tokens = line.split()
        if tokens[0] == "v":
            teapot_vertices.append([-float(tokens[1]), float(tokens[2]), float(tokens[3])])
        elif tokens[0] == "f":
            teapot_indices.append([int(token) - 1 for token in tokens[1:4]])

bunny_scene["objects"]["teapot"] = {
    "type": "mesh",
    "vertices": teapot_vertices,
    "indices": teapot_indices,
    "material": {"type": "diffuse", "reflectance": [0.35, 0.8, 0.25]}
    }

with open("bunnyetc.json", "w") as output:
    json.dump(bunny_scene, output)
