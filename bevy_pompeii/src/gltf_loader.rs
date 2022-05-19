use std::array::from_ref;
use std::path::Path;

use bevy_ecs::prelude::*;
use bevy_hierarchy::BuildWorldChildren;
use bevy_transform::TransformBundle;
use gltf::accessor::DataType;
use gltf::mesh::util::ReadIndices;
use gltf::mesh::Mode;
use gltf::Semantic;
use log::debug;

use pompeii::alloc::BufferHandle;
use pompeii::errors::{PompeiiError, Result};
use pompeii::mesh::VertexPosNormUvF32;
use pompeii::PompeiiRenderer;
use crate::mesh::{Mesh, MeshBundle, SubMesh};

// TODO: parallelize this
pub fn load_gltf_models<P: AsRef<Path>>(world: &mut World, path: P) -> Result<()> {
    let mut renderer = world.get_non_send_resource_mut::<PompeiiRenderer>().unwrap();

    debug!(
        "Loading GLTF model at {}",
        path.as_ref().to_path_buf().to_str().unwrap()
    );
    struct SubMeshIndices {
        vert_start: usize,
        vert_count: usize,
        index_start: usize,
        index_count: usize,
    }

    struct MeshIndices {
        sub_meshes: Vec<SubMeshIndices>,
    }

    let (doc, buffers, _) = gltf::import(path).map_err(|e| PompeiiError::Generic(e.to_string()))?;

    let mut meshes = Vec::new();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // TODO Refactor that to traverse the whole scene tree properly

    for mesh in doc.meshes() {
        let mut sub_meshes = Vec::new();

        for sub_mesh in mesh.primitives() {
            let vert_start = vertices.len();
            let index_start = indices.len();

            let reader = sub_mesh.reader(|buf| Some(&buffers[buf.index()]));

            // TODO: assert same amount of pos/norm/uv

            // Prepare components reader
            let pos = reader
                .read_positions()
                .ok_or(PompeiiError::NoVertexPosition)?;
            let norm = reader.read_normals().ok_or(PompeiiError::NoVertexNormal)?;
            let uv = reader.read_tex_coords(0).ok_or(PompeiiError::NoVertexUv)?;
            let index = reader.read_indices().ok_or(PompeiiError::NoModelIndices)?;

            // Transform into vertices
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

    // TODO: utiliser le vrai transform par exemple
    meshes.iter().for_each(|mesh| {
        let ent = world.spawn()
            .insert_bundle(MeshBundle::from(TransformBundle::identity()))
            .with_children(|builder| {
                for sub_mesh in &mesh.sub_meshes {
                    let &SubMeshIndices {
                        vert_start,
                        vert_count,
                        index_start,
                        index_count,
                    } = sub_mesh;

                    builder.spawn()
                        .insert(SubMesh {
                            vert_handle: vertices_handle,
                            vert_start,
                            vert_count,
                            index_handle: indices_handle,
                            index_start,
                            index_count,
                        });
                }
            });
    });

    Ok(())
}
