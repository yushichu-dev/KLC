import struct
data = open(r'.\examples\gui_demo.exe', 'rb').read()
pe_off = struct.unpack_from('<I', data, 0x3C)[0]
dd_off = pe_off + 4 + 20 + 112 + 8
idt_rva, idt_sz = struct.unpack_from('<II', data, dd_off)
print(f'Import Table RVA=0x{idt_rva:08X} Size={idt_sz}')

idt_file = 0x400 + (idt_rva - 0x2000)
for entry in range(10):
    off = idt_file + entry * 20
    oft, ts, fc, name_rva, ft = struct.unpack_from('<IIIII', data, off)
    if oft == 0 and name_rva == 0: break
    name_off = 0x400 + (name_rva - 0x2000)
    name_end = data.find(0, name_off)
    dll = data[name_off:name_end].decode()
    print(f'\n[{entry}] {dll}:')
    print(f'  OFT=0x{oft:08X} FT=0x{ft:08X}')
    # parse IAT
    iat_off = 0x400 + (ft - 0x2000)
    for j in range(20):
        thunk = struct.unpack_from('<Q', data, iat_off + j*8)[0]
        if thunk == 0: break
        if thunk >> 63:
            print(f'  IAT[{j}]: ordinal {thunk & 0xFFFF}')
        else:
            # Read Hint/Name
            hn_off = 0x400 + (thunk - 0x2000)
            hint = struct.unpack_from('<H', data, hn_off)[0]
            fn_end = data.find(0, hn_off + 2)
            fn_name = data[hn_off+2:fn_end].decode()
            print(f'  IAT[{j}]: 0x{thunk:016X} → hn=0x{thunk:08X} hint={hint} fn="{fn_name}"')
