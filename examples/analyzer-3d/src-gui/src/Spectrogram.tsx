import { Canvas, useFrame } from "@react-three/fiber";
import { OrbitControls } from "@react-three/drei";
import * as THREE from "three";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { invoke } from "@novonotes/webview-bridge";
import { useChannel } from "./channel";
import { RingBuffer2D } from "./ringBuffer";

const useSpectrogramData = () => {
    const ring = useRef(new RingBuffer2D(512, 256));

    const callback = useCallback((data: number[]) => {
        ring.current.push(data);
    }, []);

    useChannel("spectrogram", callback);


    return ring;
};

const Bars: React.FC<{ ring: RingBuffer2D }> = ({ ring }) => {
    const meshRef = useRef<THREE.InstancedMesh>(null);

    const dummy = useMemo(() => new THREE.Object3D(), []);
    const color = useMemo(() => new THREE.Color(), []);

    useFrame(() => {
        const mesh = meshRef.current;
        if (!mesh) return;

        ring.forEach((frame, x) => {
            for (let y = 0; y < ring.binCount; y++) {
                const height = frame[y] / 3;
                const idx = x * ring.binCount + y;

                dummy.position.set(-x, height / 2, y);
                dummy.scale.set(1, height, 1);
                dummy.updateMatrix();
                mesh.setMatrixAt(idx, dummy.matrix);

                color.setHSL(0.6, height, height);
                mesh.setColorAt(idx, color);
            }
        });
        mesh.instanceMatrix.needsUpdate = true;
        if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    });

    return (
        <instancedMesh
            ref={meshRef}
            args={[undefined, undefined, ring.binCount * ring.frameCount]}
        >
            <boxGeometry args={[1, 1, 1]} />
            <meshPhysicalMaterial
                roughness={0.3}
                metalness={0.0}
                clearcoat={1.0}
                clearcoatRoughness={1.0}
                ior={1.1}
                reflectivity={1.0}
                iridescenceIOR={1.0}
                specularIntensity={10}
                thickness={10.0}
                transmission={1.0}
                transparent
            />
        </instancedMesh>
    );
};

export const Spectrogram: React.FC = () => {
    const ring = useSpectrogramData();

    return (
        <Canvas
            camera={{
                position: [-300, 100, -150],

                fov: 45,
                near: 0.1,
                far: 1000,
            }}
        >
            <Bars ring={ring.current} />

            <ambientLight intensity={0.1} />
            <directionalLight position={[5, 5, 5]} color="#9999ff" intensity={2} />
            <OrbitControls
                enableDamping
                dampingFactor={0.08}
                target={[-50, 0, 300]}
            />
        </Canvas>
    );
};
