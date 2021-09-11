use <mayscadlib/positioning.scad>;
use <mayscadlib/2d/shapes.scad>;
use <./footprint_centers.scad>;

$fn=100;
wall = 2;
bot_plane=wall;
play = .2;
pcb_thickness = 1.6;
screw_length=8;
pcb_pos = screw_length-pcb_thickness;
audio_jack_dia = 5;
audio_plug_dia = 6.2;
usb_c_jack_bounds = [10, 4];
switch_cutout_size = [14.5,14.5];

mounting_positions = [
  [4.06,    -4.06],
  [129.54,  -4.06],
  [19.81,   -31.50],
  [70.10,   -32.51],
  [123.95,  -33.53],
  [128.52,  -69.60],
  [23.11,   -89.92],
  [70.10,   -106.17],
  [129.54,  -106.17]
];

audio_jacks = [
  //17.65, // Skip the left jack hole
  115.72,
];

usb_c_jack = 84.56;

footprint_origin = [69.7548, 44.547];

module outline(play=5*play) {
  offset(delta=play)
  import("./outline.dxf");
}

module wall_2d() {
  difference() {
    offset(r=wall)
    outline();
    outline();
  }
}

function screw_post_outer_rad(screw_dia=3) = ((screw_dia+play)/2)+wall;

module screw_post(screw_grip=2.5, screw_length=screw_length-pcb_thickness, screw_dia=3) {
  grip_rad = (screw_dia + play)/2;
  linear_extrude(height=screw_grip)
  difference() {
    circle(r=screw_post_outer_rad(screw_dia=screw_dia));
    circle(r=grip_rad);
  }
  lift(screw_grip)
  linear_extrude(height=screw_length-screw_grip)
  difference() {
    circle(r=grip_rad+wall);
    circle(r=grip_rad+1);
  }
}

module audio_jack_cutout() {
  cutout_h = wall + play*5;
  function rad(assumed) = (assumed+play)/2;
  translate([0,cutout_h,0])
  lift(pcb_pos-rad(audio_jack_dia)+bot_plane)
  rotate([90,0,0])
  cylinder(r2=rad(audio_plug_dia), r1=rad(audio_plug_dia*1.2), h=cutout_h);
}

module usb_c_jack_cutout() {
  cutout_h = wall + play*5;
  real_bounds = usb_c_jack_bounds + [2*play, 2*play];

  translate([0,cutout_h,0])
  lift(pcb_pos-real_bounds[1]+bot_plane)
  rotate([90,0,0])
  translate([0,real_bounds[1]/2, cutout_h])
  mirror([0,0,1])
  linear_extrude(height=cutout_h, scale=1.2)
  rounded_square(size=real_bounds, corner_rad=.5, center=true);
}

module stabilization_area() {
  difference() {
    intersection(){
      outline();
      // Cut off a bit of the top post switch area
      translate([-50,-1020])
      square(size=[1000,1000]);
    }
    place([for(k=kb_footprint_centers()) [k[0] - footprint_origin[0], -k[1]+footprint_origin[1]]])
    square(switch_cutout_size,center=true);
    place(mounting_positions)
    circle(screw_post_outer_rad());
  }
}


difference() {
  union() {

    linear_extrude(height=bot_plane) {
      wall_2d();
      outline();
    }
    translate([0,0,bot_plane])
    linear_extrude(height=pcb_pos + pcb_thickness) {
      wall_2d();
    }
  }

  place([for(x=audio_jacks) [x,0,0]])
  audio_jack_cutout();

  translate([usb_c_jack,0,0])
  usb_c_jack_cutout();
}
lift(bot_plane-.01)
linear_extrude(height=pcb_pos)
stabilization_area();

place(mounting_positions) {
  lift(bot_plane)
  screw_post();
}




