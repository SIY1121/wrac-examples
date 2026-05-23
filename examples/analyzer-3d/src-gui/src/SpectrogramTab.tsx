import { FiSettings } from "react-icons/fi";
import { Spectrogram } from "./Spectrogram";
import { useState } from "react";
import {
    Description,
    Dialog,
    DialogPanel,
    DialogTitle,
    Listbox,
    ListboxButton,
    ListboxOption,
    ListboxOptions,
} from "@headlessui/react";
import { invoke } from "@novonotes/webview-bridge";

const fft_sizes = [1024, 2048, 4096, 8192] as const;

export const SpectrogramTab = () => {
    const [isOpen, setIsOpen] = useState(false);
    const [fftSize, setFftSize] = useState<(typeof fft_sizes)[number]>(2048);

    return (
        <>
            <Spectrogram />
            <button
                className="fixed right-5 top-5 w-10 h-10 p-2 text-gray-300 rounded-full hover:bg-black/50"
                onClick={() => setIsOpen(true)}
            >
                <FiSettings size={24} />
            </button>
            <Dialog
                transition
                open={isOpen}
                onClose={() => setIsOpen(false)}
                className="transition data-closed:opacity-0"
            >
                <div className="fixed inset-0 flex w-screen items-center justify-center p-4">
                    <DialogPanel className="rounded-2xl w-full max-w-lg space-y-8 border border-white/30 bg-black/40 backdrop-blur-lg text-white p-12">
                        <DialogTitle className="font-bold text-lg">
                            Spectrogram Settings
                        </DialogTitle>
                        <Description></Description>
                        <div className="flex items-center gap-4">
                            <p>FFT Size</p>
                            <Listbox
                                value={fftSize}
                                onChange={(v) => {
                                    invoke("set_fft_size", {
                                        fftSize: v
                                    });
                                    setFftSize(v);
                                }}
                            >
                                <ListboxButton className="px-4 py-2 border border-white/30 rounded-md">
                                    {fftSize}
                                </ListboxButton>
                                <ListboxOptions
                                    anchor="bottom"
                                    className="text-white bg-black/30 backdrop-blur-lg py-2 rounded-md"
                                >
                                    {fft_sizes.map((size) => (
                                        <ListboxOption
                                            key={size}
                                            value={size}
                                            className="px-4 py-2 hover:bg-black/20"
                                        >
                                            {size}
                                        </ListboxOption>
                                    ))}
                                </ListboxOptions>
                            </Listbox>
                        </div>
                    </DialogPanel>
                </div>
            </Dialog>
        </>
    );
};
