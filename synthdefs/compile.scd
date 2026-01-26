// Run in SuperCollider to generate .scsyndef files
(
SynthDef(\tuidaw_saw, { |out=0, freq=440, amp=0.5|
    Out.ar(out, (Saw.ar(freq) * amp) ! 2);
}).writeDefFile(thisProcess.nowExecutingPath.dirname);

SynthDef(\tuidaw_sin, { |out=0, freq=440, amp=0.5|
    Out.ar(out, (SinOsc.ar(freq) * amp) ! 2);
}).writeDefFile(thisProcess.nowExecutingPath.dirname);

SynthDef(\tuidaw_sqr, { |out=0, freq=440, amp=0.5|
    Out.ar(out, (Pulse.ar(freq, 0.5) * amp) ! 2);
}).writeDefFile(thisProcess.nowExecutingPath.dirname);

SynthDef(\tuidaw_tri, { |out=0, freq=440, amp=0.5|
    Out.ar(out, (LFTri.ar(freq) * amp) ! 2);
}).writeDefFile(thisProcess.nowExecutingPath.dirname);

"Done!".postln;
)
