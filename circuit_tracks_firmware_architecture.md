# Circuit Tracks Firmware Architecture Analysis

## Overview
This document provides a comprehensive analysis of the Circuit Tracks firmware architecture based on reverse engineering of the firmware binary.

## Firmware Entry Point and Initialization

### Reset Handler (0x805c00c)
The firmware starts execution at the Reset_handler function which:
1. Calls sub_801BC78() - Sets up CPU features (FPU, memory mapping)
2. Calls sub_805B790() - Configures floating point unit
3. Calls sub_805B990() - Starts main initialization sequence

### Initialization Sequence
```
Reset_handler -> sub_805B990 -> sub_805B590 -> sub_801D3C8 -> System_InitSubsystems
```

## Main Execution Architecture

### System Manager Loop (MainLoop_SystemManager - 0x805767c)
The main system loop that:
- Initializes extended and basic system components
- Continuously processes two message queues (dword_2000696C and dword_20006968)
- Calls Hardware_Initialize in each iteration
- Never exits (infinite loop)

### Command Processing Loop (MainLoop_ProcessCommands - 0x8022ae4)
Processes queued commands with 11 different command types (0-10):

#### Command Types:
- **Command 0**: Check if drum track is active
- **Command 1**: Process drum track operation
- **Command 2**: Get drum track data and store result
- **Command 3**: Process drum track configuration
- **Command 4**: Execute drum track function
- **Command 5**: Content management operation with callback
- **Command 6**: Content read operation with result callback
- **Command 7**: Content write operation with callback
- **Command 8**: System operation with callback
- **Command 9**: Process drum track samples with optional string parameter
- **Command 10**: Content read/write operation with optional string parameter

## Key Data Structures

### CommandMessage Structure (72 bytes)
```c
struct CommandMessage {
    unsigned char command_type;      // Command type (0-10)
    unsigned char padding[3];
    unsigned int param1-param6;      // Six parameters
    unsigned int string_data[5];     // String data (20 bytes)
    unsigned int callback_func;      // Callback function pointer
    unsigned int callback_data;      // Callback data
    unsigned int result_ptr;         // Pointer to store result
};
```

### DrumTrack Structure
```c
struct DrumTrack {
    unsigned int track_id;
    unsigned int display_ptr;
    unsigned int audio_engine_ptr;
    // ... various data pointers
    unsigned char velocity_data[128];
    unsigned char note_data[256];
    unsigned char mapping_data[256];
};
```

### MessageQueue Structure
```c
struct MessageQueue {
    unsigned int vtable_ptr;
    unsigned int queue_ptr;
    unsigned int message_count;
    unsigned char lock_flag;
    unsigned int queue_size;
    unsigned int flags;
};
```

## Circuit Tracks Device Features

Based on the analysis, the firmware supports:
- **4 Drum Tracks**: Each with individual note mapping, velocity control, and sample processing
- **2 Synth Tracks**: Managed through the audio engine
- **2 MIDI Tracks**: For external MIDI communication
- **Content Management**: File system operations for samples and patterns
- **Display System**: LED control and user interface
- **Audio Engine**: DSP processing and sample playback

## Key Function Categories

### System Functions
- System_Initialize / System_InitializeExtended
- Hardware_Initialize
- Memory management (Memory_Allocate, Memory_Copy)

### Audio Functions
- DSP_Initialize, DSP_ProcessCallback
- Audio_GetSampleRate, Audio_CheckMute, Audio_CheckSolo
- Sample_ValidateWAVFormat, Sample_GetDataSize

### Drum Track Functions
- DrumTrack_IsEnabled, DrumTrack_SetNote, DrumTrack_TriggerNote
- DrumTrack_ProcessEvents, DrumTrack_ProcessStep
- DrumTrack_WriteStepData, DrumTrack_ClearStep

### MIDI Functions
- MIDI_SendToPort, MIDI_ProcessSysEx
- MIDI_StartTransaction, MIDI_SendData

### File System Functions
- FS_ValidateHeader, FS_ReadSector, FS_WriteSector
- FS_MountVolume, FS_CheckBootSector

### Display Functions
- Display_SetLED
- Hardware_EnableDisplayChannel

## Message Processing Architecture

The firmware uses a dual-queue message processing system:
1. **Extended System Queue** (dword_2000696C): Handles complex operations
2. **Basic System Queue** (dword_20006968): Handles basic system operations

Both queues are processed continuously in the main system loop, ensuring responsive operation.

## Callback System

The firmware implements an extensive callback system for asynchronous operations:
- Content operations (read/write) use callbacks to notify completion
- Drum track operations can trigger callbacks for UI updates
- System operations use callbacks for status reporting

This architecture allows the firmware to maintain responsiveness while performing complex operations like file I/O and audio processing.
