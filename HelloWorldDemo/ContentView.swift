import SwiftUI

struct ContentView: View {
    var body: some View {
        ZStack {
            VStack {
                Spacer()
                Text("Hello World")
                    .font(.system(size: 48, weight: .bold))
                    .foregroundColor(.blue)
                Spacer()
            }
        }
        .overlay(alignment: .bottomTrailing) {
            DebugOverlay()
        }
    }
}

#Preview {
    ContentView()
}
