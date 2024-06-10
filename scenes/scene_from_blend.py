"""
Creates a scene file for the renderer from a Blender project.

To use this script, run Blender in the command line from the folder where you
want to put the scene JSON and OBJ files. Command line launching is described at
docs.blender.org/manual/en/latest/advanced/command_line/launch/index.html.

Use the following arguments:
blender yourfile.blend -P scene_from_blend.py

You can also open a .blend file manually, navigate to the Scripting tab, and
load and run this script.
"""


import bpy
import math
import json
import os
import re


def xzy_to_xyz(values):
    return [values[0], values[2], values[1]]


def add_vec(a, b):
    return [x + y for (x, y) in zip(a, b)]


def multiply_quats(a, b):
    w = a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3]
    x = a[0] * b[1] + a[1] * b[0] - a[2] * b[3] + a[3] * b[2]
    y = a[0] * b[2] + a[1] * b[3] + a[2] * b[0] - a[3] * b[1]
    z = a[0] * b[3] - a[1] * b[2] + a[2] * b[1] + a[3] * b[0]
    return [w, x, y, z]


def rotate_quat(vec, quat):
    inv_quat = [quat[0]] + [-x for x in quat[1:4]]
    rotated = multiply_quats(inv_quat, multiply_quats([0] + vec, quat))
    return rotated[-3:]


scene_dict = {"camera": {}, "settings": {}, "objects": {}}
scene = bpy.context.scene

scene.camera.rotation_mode = "QUATERNION"
camera_quat = list(scene.camera.rotation_quaternion[:4])
origin = xzy_to_xyz(scene.camera.location[:3])
look_at = add_vec(origin, xzy_to_xyz(rotate_quat([0, 0, -1], camera_quat)))
up = xzy_to_xyz(rotate_quat([0, 1, 0], camera_quat))

scene_dict["camera"] = {
    "film_dimensions": [scene.render.resolution_x, scene.render.resolution_y], 
    "origin": origin,
    "look_at": look_at,
    "up": up,
    "field_of_view": math.degrees(scene.camera.data.angle)
    }

environment = [0.8] * 3
if "Background" in scene.world.node_tree.nodes:
    background_node = scene.world.node_tree.nodes["Background"]
    environment = list(background_node.inputs[0].default_value[:3])

scene_dict["settings"] = {
    "environment": environment,
    "samples_per_pixel": scene.cycles.samples,
    "max_ray_depth": scene.cycles.max_bounces
    }

bpy.ops.object.select_all(action="DESELECT")
export_args = {
    "forward_axis": "Z",
    "up_axis": "Y",
    "export_selected_objects": True,
    "export_triangulated_mesh": True, 
    "export_uv": False, 
    "export_normals": False, 
    "export_materials": False
    }

os.makedirs("objects", exist_ok=True)
meshes = [m for m in bpy.data.objects if m.type == "MESH" and not m.hide_render]
for mesh in meshes:
    mesh.select_set(True)
    bpy.ops.wm.obj_export(filepath=f"objects/{mesh.name}.obj", **export_args)
    mesh.select_set(False)

    color = [0.5] * 3
    materials = mesh.data.materials.values()
    if len(materials) > 0 and "Diffuse BSDF" in materials[0].node_tree.nodes:
        diffuse_node = materials[0].node_tree.nodes["Diffuse BSDF"]
        color = list(diffuse_node.inputs[0].default_value[:3])

    scene_dict["objects"][mesh.name] = {
        "type": "mesh",
        "file": f"{os.getcwd()}/objects/{mesh.name}.obj",
        "material": {"type": "diffuse", "color": color}
        }

blend_name = bpy.path.basename(bpy.data.filepath)
output_name = blend_name[:-6] if blend_name.endswith(".blend") else "untitled"
output_filename = f"{output_name}.json"
for i in range(1, 100):
    if os.path.exists(output_filename):
        output_filename = f"{output_name}_{i}.json"
    else:
        break

number_pattern = "-?[0-9]+(?:\.[0-9]+)?"
double_pattern = "\[\n *({0}),\n *({0})\n *\]".format(number_pattern)
triple_pattern = "\[\n *({0}),\n *({0}),\n *({0})\n *\]".format(number_pattern)
scene_string = json.dumps(scene_dict, indent=4)
scene_string = re.sub(double_pattern, r"[\1, \2]", scene_string)
scene_string = re.sub(triple_pattern, r"[\1, \2, \3]", scene_string)

with open(output_filename, "w") as file:
    file.write(scene_string)

print(f"Created scene file at {output_filename}")
