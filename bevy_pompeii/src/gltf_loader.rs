use gltf::mesh::Mode;
use log::debug;

pub fn load_gltf() {
    let (doc, buffers, _) = gltf::import("./assets/BetterCube.glb").unwrap();

    // Get first mesh
    let mesh = doc.meshes().next().unwrap();
    // Get first primitives
    let sub_mesh = mesh.primitives().next().unwrap();
    let reader = sub_mesh.reader(|buf| Some(&buffers[buf.index()]));
    assert_eq!(sub_mesh.mode(), Mode::Triangles);

    // Read positions
    let pos = reader.read_positions().unwrap().collect::<Vec<_>>();
    let normals = reader.read_normals().unwrap().collect::<Vec<_>>();
    let uvs = reader.read_tex_coords(0).unwrap().into_f32().collect::<Vec<_>>();
    let indices = reader.read_indices().unwrap().into_u32().collect::<Vec<_>>();
}
