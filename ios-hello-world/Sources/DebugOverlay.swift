import SwiftUI
import QuartzCore

// Invisible UIKit view that uses CADisplayLink to tick every frame
struct FrameCounter: UIViewRepresentable {
    @Binding var frameCount: Int

    func makeUIView(context: Context) -> UIView {
        let view = UIView()
        let displayLink = CADisplayLink(target: context.coordinator, selector: #selector(Coordinator.tick))
        displayLink.add(to: .main, forMode: .common)
        context.coordinator.displayLink = displayLink
        return view
    }

    func updateUIView(_ uiView: UIView, context: Context) {}

    func makeCoordinator() -> Coordinator {
        Coordinator(frameCount: $frameCount)
    }

    class Coordinator: ObservableObject {
        var displayLink: CADisplayLink?
        @Binding var frameCount: Int

        init(frameCount: Binding<Int>) {
            _frameCount = frameCount
        }

        @objc func tick() {
            frameCount += 1
        }

        deinit {
            displayLink?.invalidate()
        }
    }
}

struct DebugOverlay: View {
    @State private var frameCount: Int = 0
    @State private var fps: Double = 0

    let deviceName: String = {
        var systemInfo = utsname()
        uname(&systemInfo)
        let machineMirror = Mirror(reflecting: systemInfo.machine)
        let identifier = machineMirror.children.reduce("") { identifier, element in
            guard let value = element.value as? Int8, value != 0 else { return identifier }
            return identifier + String(UnicodeScalar(UInt8(value)))
        }
        return identifier
    }()

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("FPS: \(String(format: "%.0f", fps))")
                .font(.system(size: 12, design: .monospaced))
            Text("Frame: \(frameCount)")
                .font(.system(size: 12, design: .monospaced))
            Text("Depth: 1")
                .font(.system(size: 12, design: .monospaced))
            Text("Device: \(deviceName)")
                .font(.system(size: 10, design: .monospaced))
                .lineLimit(1)
        }
        .padding(8)
        .background(Color.black.opacity(0.6))
        .foregroundColor(.white)
        .cornerRadius(6)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .padding(8)
        .allowsHitTesting(false)
        .overlay {
            FrameCounter(frameCount: $frameCount)
                .frame(width: 0, height: 0)
                .opacity(0)
        }
        .onReceive(Timer.publish(every: 1.0, on: .main, in: .common).autoconnect()) { _ in
            fps = Double(frameCount)
            frameCount = 0
        }
    }
}

#Preview {
    ZStack {
        Color.gray
        DebugOverlay()
    }
}
