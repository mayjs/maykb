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

footprint_origin = [28.730000, 52.324000];
mounting_positions = [ for(p=kb_mount_centers()) apply_origin(p) ];

// If you want to have both audio jacks, use this line
// audio_jacks = [ for(x=audio_jack_x()) x - footprint_origin[0] ];
// If you only need the left one, use this:
audio_jacks = [ audio_jack_x()[0] - footprint_origin[0] ];

usb_c_jack = usb_c_x()[0] - footprint_origin[0];

pcb_play = 5*play;

module outline(play=pcb_play) {
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

function apply_origin(pos) = [pos[0] - footprint_origin[0], -pos[1]+footprint_origin[1]];

module screw_post(screw_grip=2, screw_length=screw_length-pcb_thickness, screw_dia=3) {
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
    translate([pcb_play,-pcb_play,0])
    place([for(k=kb_footprint_centers()) apply_origin(k)])
    square(switch_cutout_size,center=true);
    translate([pcb_play,-pcb_play,0])
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

  translate([pcb_play,-pcb_play]) {
    place([for(x=audio_jacks) [x,0,0]])
    audio_jack_cutout();

    translate([usb_c_jack,0,0])
    usb_c_jack_cutout();
  }
}
lift(bot_plane-.01)
linear_extrude(height=pcb_pos)
stabilization_area();

translate([pcb_play,-pcb_play])
place(mounting_positions) {
  lift(bot_plane)
  screw_post();
}
