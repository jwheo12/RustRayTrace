mod aabb;
mod bvh;
mod camera;
mod color;
mod hittable;
mod hittable_list;
mod interval;
mod material;
mod ray;
mod rtweekend;
mod sphere;
mod vec3;

use std::sync::Arc;

use bvh::BvhNode;
use camera::Camera;
use hittable::make_ref;
use hittable_list::HittableList;
use material::{Dielectric, Lambertian, Metal};
use rtweekend::random_double;
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
}

pub fn run(_scene: Option<i32>) {
    let mut world = HittableList::new();

    let ground_material: Arc<dyn material::Material + Send + Sync> =
        Arc::new(Lambertian::new(Color::new(0.5, 0.5, 0.5)));
    world.add(make_ref(Sphere::new(
        Point3::new(0.0, -1000.0, 0.0),
        1000.0,
        ground_material,
    )));

    for a in -11..11 {
        for b in -11..11 {
            let choose_mat = random_double();
            let center = Point3::new(
                a as f64 + 0.9 * random_double(),
                0.2,
                b as f64 + 0.9 * random_double(),
            );

            if (center - Point3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                let sphere_material: Arc<dyn material::Material + Send + Sync>;

                if choose_mat < 0.8 {
                    // diffuse
                    let albedo = Color::random() * Color::random();
                    sphere_material = Arc::new(Lambertian::new(albedo));
                } else if choose_mat < 0.95 {
                    // metal
                    let albedo = Color::random_range(0.5, 1.0);
                    let fuzz = random_double() * 0.5;
                    sphere_material = Arc::new(Metal::new(albedo, fuzz));
                } else {
                    // glass
                    sphere_material = Arc::new(Dielectric::new(1.5));
                }

                world.add(make_ref(Sphere::new(center, 0.2, sphere_material)));
            }
        }
    }

    let material1: Arc<dyn material::Material + Send + Sync> = Arc::new(Dielectric::new(1.5));
    world.add(make_ref(Sphere::new(Point3::new(0.0, 1.0, 0.0), 1.0, material1)));

    let material2: Arc<dyn material::Material + Send + Sync> =
        Arc::new(Lambertian::new(Color::new(0.4, 0.2, 0.1)));
    world.add(make_ref(Sphere::new(Point3::new(-4.0, 1.0, 0.0), 1.0, material2)));

    let material3: Arc<dyn material::Material + Send + Sync> =
        Arc::new(Metal::new(Color::new(0.7, 0.6, 0.5), 0.0));
    world.add(make_ref(Sphere::new(Point3::new(4.0, 1.0, 0.0), 1.0, material3)));

    let mut cam = Camera::default();

    cam.aspect_ratio = 16.0 / 9.0;
    cam.image_width = 1200;
    cam.samples_per_pixel = 10;
    cam.max_depth = 20;

    cam.vfov = 20.0;
    cam.lookfrom = Point3::new(13.0, 2.0, 3.0);
    cam.lookat = Point3::new(0.0, 0.0, 0.0);
    cam.vup = Vec3::new(0.0, 1.0, 0.0);

    cam.defocus_angle = 0.6;
    cam.focus_dist = 10.0;

    apply_overrides(&mut cam);

    let world = BvhNode::new(world);
    cam.render(&world);
}
