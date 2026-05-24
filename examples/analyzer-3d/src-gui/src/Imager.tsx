import { invoke } from "@novonotes/webview-bridge";
import { OrbitControls } from "@react-three/drei";
import { Canvas, useFrame } from "@react-three/fiber";
import { use, useCallback, useEffect, useMemo, useRef } from "react";
import * as THREE from "three";
import { RingBuffer } from "./ringBuffer";
import { useChannel } from "./channel";

type Event = {
    type: "imager-update";
    values: number[];
};

const useImagerData = (size: number) => {
    const ring = useRef(new RingBuffer(size * 2));

    const callback = useCallback((data: number[]) => {
        for (let i = 0; i < data.length; i++) {
            ring.current.push(data[i]);
        }
    }, []);

    useChannel("imager", callback);

    return ring;
};

const size = 512;

export const Points: React.FC = () => {
    const ring = useImagerData(size);
    const ref = useRef<THREE.Points>(null);
    const buf = useMemo(() => new Float32Array(size * 3), []);
    const colors = useMemo(() => new Float32Array(size * 3), []);
    const dummyColor = useMemo(() => new THREE.Color(), []);

    useFrame(() => {
        if (!ref.current) return;

        for (let i = 0; i < ring.current.length / 2; i++) {
            const x = ring.current.read(i * 2);
            const y = ring.current.read(i * 2 + 1);

            buf[i * 3] = x * 10;
            buf[i * 3 + 1] = (x * x + y * y) * 10;
            buf[i * 3 + 2] = y * 10;

            dummyColor.setHSL(Math.sqrt(x * x + y * y) * 2, 1.0, 0.5);
            colors[i * 3] = dummyColor.r;
            colors[i * 3 + 1] = dummyColor.g;
            colors[i * 3 + 2] = dummyColor.b;
        }

        ref.current.geometry.attributes.position.needsUpdate = true;
        ref.current.geometry.attributes.color.needsUpdate = true;
    });

    return (
        <points ref={ref}>
            <bufferGeometry>
                <bufferAttribute
                    attach="attributes-position"
                    args={[buf, 3]}
                />
                <bufferAttribute attach="attributes-color" args={[colors, 3]} />
            </bufferGeometry>
            {/* <pointsMaterial size={0.05} color="orange" /> */}
            <pointsMaterial vertexColors size={0.1} />
        </points>
    );
};

export const Imager: React.FC = () => {
    return (
        <Canvas
            camera={{
                position: [5, 5, 5],

                fov: 45,
                near: 0.1,
                far: 1000,
            }}
        >
            <Points />
            <ambientLight intensity={0.1} />
            <directionalLight position={[5, 5, 5]} color="#9999ff" intensity={2} />
            <directionalLight
                position={[-5, -5, -5]}
                color="#9999ff"
                intensity={0.5}
            />
            <gridHelper args={[10, 20]} />
            <OrbitControls enableDamping dampingFactor={0.08} />
        </Canvas>
    );
};
