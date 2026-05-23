import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/react";
import { SpectrogramTab } from "./SpectrogramTab";
import { ImagerTab } from "./ImagerTab";
import { useEditorPage } from "./page";

const GlassTab = ({ children }: { children: React.ReactNode }) => (
  <Tab className="
      transition-colors
      bg-black/20
      backdrop-blur-xl
      inset-shadow-amber-500/50
      text-gray-300
      rounded-full
      px-4 py-2
      data-selected:bg-white/5
      data-selected:inset-shadow-md
      outline-none
      data-selected:text-white">
    {children}
  </Tab>
);

function App() {

  const { page, setPage } = useEditorPage();

  return (
    <div className="w-full min-h-screen h-screen bg-radial-[at_50%_-30%] from-dark-blue to-black">
      <TabGroup
        selectedIndex={page === "spectrogram" ? 0 : 1}
        onChange={((index) => {
          if (index === 0) {
            setPage("spectrogram");
          } else if (index === 1) {
            setPage("imager");
          }
        })}
        className="w-full h-full">
        <TabList className="
          fixed z-10 top-2 left-1/2 -translate-x-1/2
          p-4 rounded-full flex justify-center gap-4
          border border-white/30">
          <GlassTab>
            Spectrogram
          </GlassTab>
          <GlassTab>
            Imager
          </GlassTab>
        </TabList>
        <TabPanels className="w-full h-full">
          <TabPanel className="w-full h-full">
            <SpectrogramTab />
          </TabPanel>
          <TabPanel className="w-full h-full">
            <ImagerTab />
          </TabPanel>
        </TabPanels>
      </TabGroup>
    </div>
  );
}

export default App;
