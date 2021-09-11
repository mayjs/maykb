import fileinput

if __name__ == "__main__":
    active = False
    switches = []
    mounts = []
    audio_jacks = []
    usb = []
    for line in fileinput.input():
        if line.startswith("footprint"):
            if "Cherry" in line:
                active = "switch"
            elif "MountingHole" in line:
                active = "mount"
            elif "TRRS" in line:
                active = "audio"
            elif "USB_C_Receptacle" in line:
                active = "usb"

        if line.startswith("position") and active:
            parts = line.split(" ")
            x = float(parts[1])
            y = float(parts[2])
            if active == "switch":
                switches.append((x,y))
            elif active == "mount":
                mounts.append((x,y))
            elif active == "audio":
                audio_jacks.append((x,y))
            elif active == "usb":
                usb.append((x,y))
            active = False
    print("function kb_footprint_centers() = [")
    for x,y in switches:
        print(f"    [{x},{y}],")
    print("];")
    print("")
    print("function kb_mount_centers() = [")
    for x,y in mounts:
        print(f"    [{x},{y}],")
    print("];")
    print("")
    print("function audio_jack_x() = [")
    for x,y in audio_jacks:
        print(f"    {x},")
    print("];")
    print("")
    print("function usb_c_x() = [")
    for x,y in usb:
        print(f"    {x},")
    print("];")

