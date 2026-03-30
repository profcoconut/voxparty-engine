import SwiftUI

struct ContentView: View {
    @State private var counter = 0

    var body: some View {
        VStack(spacing: 20) {
            Text("Hello, World!")
                .font(.largeTitle)
                .fontWeight(.bold)

            Text("Tap count: \(counter)")
                .font(.title2)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .contentShape(Rectangle())
        .onTapGesture {
            counter += 1
        }
        .overlay(alignment: .topLeading) {
            DebugOverlay()
        }
    }
}

#Preview {
    ContentView()
}
