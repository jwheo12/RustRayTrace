mod aabb;
mod bvh;
mod camera;
mod color;
mod constant_medium;
mod hittable;
mod hittable_list;
mod interval;
mod material;
mod onb;
mod pdf;
mod perlin;
mod quad;
mod ray;
mod rtw_image;
mod rtweekend;
mod sphere;
mod texture;
mod vec3;

use std::sync::Arc;

use bvh::BvhNode;
use camera::Camera;
use hittable::{RotateY, Translate};
use hittable_list::HittableList;
use material::{Dielectric, DiffuseLight, EmptyMaterial, Lambertian};
use quad::{make_box, Quad};
use sphere::Sphere;
use vec3::{Color, Point3, Vec3};

fn apply_overrides(cam: &mut Camera) {
    let o = crate::config::OVERRIDES;
    if let Some(value) = o.aspect_ratio {
        cam.aspect_ratio = value;
    }
    if let Some(value) = o.image_width {
        cam.image_width = value;
    }
    if let Some(value) = o.samples_per_pixel {
        cam.samples_per_pixel = value;
    }
    if let Some(value) = o.max_depth {
        cam.max_depth = value;
    }
    if let Some(value) = o.vfov {
        cam.vfov = value;
    }
    if let Some(value) = o.lookfrom {
        cam.lookfrom = Point3::new(value[0], value[1], value[2]);
    }
    if let Some(value) = o.lookat {
        cam.lookat = Point3::new(value[0], value[1], value[2]);
    }
    if let Some(value) = o.vup {
        cam.vup = Vec3::new(value[0], value[1], value[2]);
    }
    if let Some(value) = o.defocus_angle {
        cam.defocus_angle = value;
    }
    if let Some(value) = o.focus_dist {
        cam.focus_dist = value;
    }
    if let Some(value) = o.background {
        cam.background = Color::new(value[0], value[1], value[2]);
    }
}

pub fn run(_scene: Option<i32>) {
    let mut world = HittableList::new();

    let red = Arc::new(Lambertian::new(Color::new(0.65, 0.05, 0.05)));
    let white = Arc::new(Lambertian::new(Color::new(0.73, 0.73, 0.73)));
    let green = Arc::new(Lambertian::new(Color::new(0.12, 0.45, 0.15)));
    let light = Arc::new(DiffuseLight::new(Color::new(15.0, 15.0, 15.0)));

    // Cornell box sides
    world.add(Arc::new(Quad::new(
        Point3::new(555.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 555.0),
        Vec3::new(0.0, 555.0, 0.0),
        green,
    )));
    world.add(Arc::new(Quad::new(
        Point3::new(0.0, 0.0, 555.0),
        Vec3::new(0.0, 0.0, -555.0),
        Vec3::new(0.0, 555.0, 0.0),
        red,
    )));
    world.add(Arc::new(Quad::new(
        Point3::new(0.0, 555.0, 0.0),
        Vec3::new(555.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 555.0),
        white.clone(),
    )));
    world.add(Arc::new(Quad::new(
        Point3::new(0.0, 0.0, 555.0),
        Vec3::new(555.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -555.0),
        white.clone(),
    )));
    world.add(Arc::new(Quad::new(
        Point3::new(555.0, 0.0, 555.0),
        Vec3::new(-555.0, 0.0, 0.0),
        Vec3::new(0.0, 555.0, 0.0),
        white.clone(),
    )));

    // Light
    world.add(Arc::new(Quad::new(
        Point3::new(213.0, 554.0, 227.0),
        Vec3::new(130.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 105.0),
        light,
    )));

    // Box
    let box1 = make_box(Point3::new(0.0, 0.0, 0.0), Point3::new(165.0, 330.0, 165.0), white.clone());
    let box1 = Arc::new(RotateY::new(box1, 15.0));
    let box1 = Arc::new(Translate::new(box1, Vec3::new(265.0, 0.0, 295.0)));
    world.add(box1);

    // Glass Sphere
    let glass = Arc::new(Dielectric::new(1.5));
    world.add(Arc::new(Sphere::new(Point3::new(190.0, 90.0, 190.0), 90.0, glass)));

    // Light Sources
    let empty_material = Arc::new(EmptyMaterial);
    let mut lights = HittableList::new();
    lights.add(Arc::new(Quad::new(
        Point3::new(343.0, 554.0, 332.0),
        Vec3::new(-130.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -105.0),
        empty_material.clone(),
    )));
    lights.add(Arc::new(Sphere::new(
        Point3::new(190.0, 90.0, 190.0),
        90.0,
        empty_material,
    )));

    let mut cam = Camera::default();

    cam.aspect_ratio = 1.0;
    cam.image_width = 600;
    cam.samples_per_pixel = 100;
    cam.max_depth = 50;
    cam.background = Color::new(0.0, 0.0, 0.0);

    cam.vfov = 40.0;
    cam.lookfrom = Point3::new(278.0, 278.0, -800.0);
    cam.lookat = Point3::new(278.0, 278.0, 0.0);
    cam.vup = Vec3::new(0.0, 1.0, 0.0);

    cam.defocus_angle = 0.0;

    apply_overrides(&mut cam);

    let world = BvhNode::new(world);
    cam.render(&world, Arc::new(lights));
}
