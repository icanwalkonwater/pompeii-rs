use std::path::Path;

use gltf::accessor::DataType;
use gltf::mesh::util::ReadIndices;
use gltf::mesh::Mode;
use gltf::Semantic;
use log::debug;

use pompeii::errors::{PompeiiError, Result};
use pompeii::mesh::VertexPosNormUvF32;
use pompeii::PompeiiRenderer;

// TODO: parallelize this
pub fn load_gltf_models<P: AsRef<Path>>(renderer: &mut PompeiiRenderer, path: P) -> Result<()> {
    struct SubMeshIndices {
        vert_start: usize,
        vert_count: usize,
        index_start: usize,
        index_count: usize,
    }

    struct MeshIndices {
        sub_meshes: Vec<SubMeshIndices>,
    }

    let (doc, buffers, _) = gltf::import(path)?;

    let mut meshes = Vec::new();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for mesh in doc.meshes() {
        let mut sub_meshes = Vec::new();

        for sub_mesh in mesh.primitives() {
            let vert_start = vertices.len();
            let index_start = indices.len();

            let reader = sub_mesh.reader(|buf| Some(&buffers[buf.index()]));

            let pos = reader
                .read_positions()
                .ok_or(PompeiiError::NoVertexPosition)?;
            let norm = reader.read_normals().ok_or(PompeiiError::NoVertexNormal)?;
            let uv = reader.read_tex_coords(0).ok_or(PompeiiError::NoVertexUv)?;
            let index = reader.read_indices().ok_or(PompeiiError::NoModelIndices)?;

            for (((pos, norm), uv), index) in pos.zip(norm).zip(uv.into_f32()).zip(index.into_u32())
            {
                vertices.push(VertexPosNormUvF32 { pos, norm, uv });
                indices.push(index as u16);
            }

            let vert_count = vertices.len() - vert_start;
            let index_count = indices.len() - index_start;

            sub_meshes.push(SubMeshIndices {
                vert_start,
                vert_count,
                index_start,
                index_count,
            });
        }

        meshes.push(MeshIndices { sub_meshes });
    }

    let mut transfer_ctx = renderer.start_transfer_operations();
    let vertices_handle = transfer_ctx.create_vertex_buffer(&vertices)?;
    let indices_handle = transfer_ctx.create_index_buffer(&indices)?;
    transfer_ctx.submit_and_wait()?;

    let meshes = meshes.into_iter().map(|mesh| {
        todo!();
    });

    Ok(())
}

pub fn load_gltf_old() {
    let (doc, buffers, _) = gltf::import("./assets/BetterCube.glb").unwrap();

    // Get first mesh
    let mesh = doc.meshes().next().unwrap();
    // Get first primitives
    let sub_mesh = mesh.primitives().next().unwrap();
    let reader = sub_mesh.reader(|buf| Some(&buffers[buf.index()]));
    assert_eq!(sub_mesh.mode(), Mode::Triangles);

    let pos_raw = {
        let accessor = sub_mesh.get(&Semantic::Positions).unwrap();
        let v = accessor.view().unwrap();
        dbg!(v.buffer().index(), v.offset(), v.length());
        &buffers[v.buffer().index()].0[v.offset()..v.offset() + v.length()]
    };

    let norm_raw = {
        let accessor = sub_mesh.get(&Semantic::Normals).unwrap();
        let v = accessor.view().unwrap();
        dbg!(v.buffer().index(), v.offset(), v.length());
        &buffers[v.buffer().index()].0[v.offset()..v.offset() + v.length()]
    };

    let uvs_raw = {
        let accessor = sub_mesh.get(&Semantic::TexCoords(0)).unwrap();
        let v = accessor.view().unwrap();
        dbg!(v.buffer().index(), v.offset(), v.length());
        &buffers[v.buffer().index()].0[v.offset()..v.offset() + v.length()]
    };

    let indices_raw = {
        let accessor = sub_mesh.indices().unwrap();
        assert_eq!(accessor.data_type(), DataType::U16);
        let v = accessor.view().unwrap();
        dbg!(v.buffer().index(), v.offset(), v.length());
        &buffers[v.buffer().index()].0[v.offset()..v.offset() + v.length()]
    };

    // Read positions
    let pos = reader.read_positions().unwrap().collect::<Vec<_>>();
    let normals = reader.read_normals().unwrap().collect::<Vec<_>>();
    let uvs = reader
        .read_tex_coords(0)
        .unwrap()
        .into_f32()
        .collect::<Vec<_>>();
    let indices = {
        let reader = reader.read_indices().unwrap();
        if let ReadIndices::U16(indices) = reader {
            indices.collect::<Vec<_>>()
        } else {
            panic!();
        }
    };

    assert_eq!(pos.len() * std::mem::size_of::<[f32; 3]>(), pos_raw.len());
    assert_eq!(&pos, unsafe {
        let (before, data, after) = pos_raw.align_to::<[f32; 3]>();
        assert!(before.is_empty());
        assert!(after.is_empty());
        data
    });

    assert_eq!(
        normals.len() * std::mem::size_of::<[f32; 3]>(),
        norm_raw.len()
    );
    assert_eq!(&normals, unsafe {
        let (before, data, after) = norm_raw.align_to::<[f32; 3]>();
        assert!(before.is_empty());
        assert!(after.is_empty());
        data
    });

    assert_eq!(uvs.len() * std::mem::size_of::<[f32; 2]>(), uvs_raw.len());
    assert_eq!(&uvs, unsafe {
        let (before, data, after) = uvs_raw.align_to::<[f32; 2]>();
        assert!(before.is_empty());
        assert!(after.is_empty());
        data
    });

    assert_eq!(
        indices.len() * std::mem::size_of::<u16>(),
        indices_raw.len()
    );
    assert_eq!(&indices, unsafe {
        let (before, data, after) = indices_raw.align_to::<u16>();
        assert!(before.is_empty());
        assert!(after.is_empty());
        data
    });

    debug!("Nice");
}
